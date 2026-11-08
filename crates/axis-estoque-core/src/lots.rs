use crate::{
    model::*,
    stock::{assert_unlocked, audit, movement, pool_balance, replayed},
    storage::Store,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

pub(crate) fn validate_lot(lot: &LotDraft) -> Result<()> {
    text(&lot.label, "Identificação do lote", 100, true)?;
    quantity(lot.quantity, true)?;
    if let Some(d) = &lot.manufactured_on {
        date(d)?;
    }
    if let Some(d) = &lot.expires_on {
        date(d)?;
    }
    if lot
        .manufactured_on
        .as_ref()
        .zip(lot.expires_on.as_ref())
        .is_some_and(|(a, b)| a > b)
    {
        return Err(CoreError::invalid(
            "A validade não pode ser anterior à fabricação.",
        ));
    }
    if lot.vintage.is_some_and(|v| !(1800..=2200).contains(&v)) {
        return Err(CoreError::invalid("Safra deve estar entre 1800 e 2200."));
    }
    Ok(())
}
pub fn expiry_alert(expires: Option<&str>, today: &str, settings: &Settings) -> Result<String> {
    let Some(expires) = expires else {
        return Ok("undated".into());
    };
    let days = (date(expires)? - date(today)?).num_days();
    Ok(if days < 0 {
        "expired"
    } else if days <= settings.short_days {
        "short"
    } else if days <= settings.long_days {
        "long"
    } else {
        "safe"
    }
    .into())
}
impl Store {
    pub fn lots(&self, today: &str) -> Result<Vec<Lot>> {
        date(today)?;
        let settings = self.settings()?;
        let mut q=self.db.connection.prepare("SELECT l.id,l.sku_id,p.name,s.internal_code,coalesce(c.name,''),l.label,l.manufactured_on,l.expires_on,l.vintage,l.supplier_id,coalesce(f.name,''),l.receipt_id,l.received_quantity,coalesce(b.balance,0),l.unit_cost_cents FROM lots l JOIN skus s ON s.id=l.sku_id JOIN products p ON p.id=s.product_id LEFT JOIN categories c ON c.id=p.category_id LEFT JOIN suppliers f ON f.id=l.supplier_id LEFT JOIN (SELECT lot_id,sum(quantity) balance FROM stock_movements WHERE lot_id IS NOT NULL GROUP BY lot_id) b ON b.lot_id=l.id ORDER BY l.expires_on IS NULL,l.expires_on,l.id")?;
        let mut lots = q
            .query_map([], |r| {
                Ok(Lot {
                    id: r.get(0)?,
                    sku_id: r.get(1)?,
                    sku_name: r.get(2)?,
                    internal_code: r.get(3)?,
                    category: r.get(4)?,
                    label: r.get(5)?,
                    manufactured_on: r.get(6)?,
                    expires_on: r.get(7)?,
                    vintage: r.get(8)?,
                    supplier_id: r.get(9)?,
                    supplier_name: r.get(10)?,
                    receipt_id: r.get(11)?,
                    received_quantity: r.get(12)?,
                    balance: r.get(13)?,
                    unit_cost_cents: r.get(14)?,
                    alert: String::new(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for lot in &mut lots {
            lot.alert = expiry_alert(lot.expires_on.as_deref(), today, &settings)?;
        }
        Ok(lots)
    }
    pub fn plan_issue(&self, input: IssuePlanInput) -> Result<IssuePlan> {
        quantity(input.quantity, true)?;
        assert_unlocked(&self.db.connection, input.sku_id)?;
        let tracked: bool = self
            .db
            .connection
            .query_row(
                "SELECT expiry_control FROM skus WHERE id=?1",
                [input.sku_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| CoreError::invalid("SKU não encontrado."))?;
        let today = local_date();
        let lots = self.lots(&today)?;
        let mut remaining = input.quantity;
        let mut allocations = vec![];
        let mut warnings = vec![];
        // lots() já ordena datas reais primeiro; desconhecidos só entram por decisão explícita.
        for lot in lots
            .iter()
            .filter(|l| l.sku_id == input.sku_id && l.balance > 0)
        {
            if lot.alert == "expired" && !input.allow_expired {
                continue;
            }
            if tracked && lot.expires_on.is_none() && !input.include_undated {
                continue;
            }
            let q = remaining.min(lot.balance);
            if q > 0 {
                allocations.push(LotQuantity {
                    lot_id: Some(lot.id),
                    quantity: q,
                });
                remaining -= q;
                if lot.alert == "expired" {
                    warnings.push(format!(
                        "Lote {} vencido: retirada exige confirmação explícita.",
                        lot.label
                    ));
                }
                if lot.expires_on.is_none() {
                    warnings.push(format!("Lote {} sem validade informada.", lot.label));
                }
            }
        }
        if remaining > 0 && (!tracked || input.include_undated) {
            let q = remaining.min(pool_balance(&self.db.connection, input.sku_id, None)?);
            if q > 0 {
                allocations.push(LotQuantity {
                    lot_id: None,
                    quantity: q,
                });
                remaining -= q;
                warnings.push(
                    "Inclui saldo sem lote identificado, após os lotes datados elegíveis.".into(),
                );
            }
        }
        if remaining > 0 {
            return Err(CoreError::invalid("Saldo elegível insuficiente. Confira os lotes e as decisões sobre vencidos e ausência de validade."));
        }
        Ok(IssuePlan {
            sku_id: input.sku_id,
            quantity: input.quantity,
            allocations,
            warnings,
        })
    }
    pub fn regularize_lot(&mut self, input: RegularizeInput) -> Result<()> {
        if !input.confirmed {
            return Err(CoreError::invalid(
                "Confirme a regularização do saldo sem lote.",
            ));
        }
        validate_lot(&input.lot)?;
        let reason = text(&input.reason, "Motivo", 500, true)?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if replayed(&tx, &input.request_key, "regularize", &input)? {
            return Ok(());
        }
        assert_unlocked(&tx, input.sku_id)?;
        let cost: i64 = tx
            .query_row(
                "SELECT purchase_cents FROM skus WHERE id=?1",
                [input.sku_id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| CoreError::invalid("SKU não encontrado."))?;
        tx.execute("INSERT INTO lots(sku_id,label,manufactured_on,expires_on,vintage,received_quantity,unit_cost_cents) VALUES(?1,?2,?3,?4,?5,0,?6)",params![input.sku_id,input.lot.label,input.lot.manufactured_on,input.lot.expires_on,input.lot.vintage,cost])?;
        let lot = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO stock_operations(request_key,kind,reason) VALUES(?1,'reclassify',?2)",
            params![input.request_key, reason],
        )?;
        movement(
            &tx,
            crate::stock::StockWrite {
                sku: input.sku_id,
                lot: None,
                quantity: -input.lot.quantity,
                kind: "reclassify",
                origin_type: "reclassify",
                origin_id: &input.request_key,
                origin_line: 0,
                reason: &reason,
                unit_cost_cents: cost,
            },
        )?;
        movement(
            &tx,
            crate::stock::StockWrite {
                sku: input.sku_id,
                lot: Some(lot),
                quantity: input.lot.quantity,
                kind: "reclassify",
                origin_type: "reclassify",
                origin_id: &input.request_key,
                origin_line: 1,
                reason: &reason,
                unit_cost_cents: cost,
            },
        )?;
        audit(&tx, "lot_regularized", input.sku_id, &reason)?;
        tx.commit()?;
        Ok(())
    }
}
