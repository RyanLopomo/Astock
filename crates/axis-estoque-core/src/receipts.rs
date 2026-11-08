use crate::{
    catalog::resolve,
    decimal::{unit_cost, Decimal},
    lots::validate_lot,
    model::*,
    stock::{active_sku, assert_unlocked, audit, movement},
    storage::Store,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

const SUMMARY: &str = "SELECT r.id,r.supplier_id,coalesce(s.name,''),r.status,r.source,x.invoice_number,x.invoice_series,x.issued_at,x.access_key,r.notes,r.revision,r.created_at,coalesce((SELECT sum(total_cents) FROM receipt_items WHERE receipt_id=r.id),0) FROM receipts r LEFT JOIN suppliers s ON s.id=r.supplier_id LEFT JOIN purchase_xml_imports x ON x.receipt_id=r.id";
fn summary_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ReceiptSummary> {
    Ok(ReceiptSummary {
        id: r.get(0)?,
        supplier_id: r.get(1)?,
        supplier_name: r.get(2)?,
        status: r.get(3)?,
        source: r.get(4)?,
        invoice_number: r.get(5)?,
        invoice_series: r.get(6)?,
        issued_at: r.get(7)?,
        access_key: r.get(8)?,
        notes: r.get(9)?,
        revision: r.get(10)?,
        created_at: r.get(11)?,
        total_cents: r.get(12)?,
    })
}
pub(crate) fn draft(c: &Connection, id: i64, revision: i64) -> Result<()> {
    let (status, current): (String, i64) = c
        .query_row(
            "SELECT status,revision FROM receipts WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| CoreError::invalid("Recebimento não encontrado."))?;
    if status != "draft" {
        return Err(CoreError::invalid("O recebimento já foi encerrado."));
    }
    if revision != current {
        return Err(CoreError::new(
            "STALE_REVISION",
            "O recebimento foi alterado. Atualize e confira novamente.",
        ));
    }
    Ok(())
}
pub(crate) fn detail(c: &Connection, id: i64) -> Result<ReceiptDetail> {
    let receipt = c
        .query_row(&format!("{SUMMARY} WHERE r.id=?1"), [id], summary_row)
        .optional()?
        .ok_or_else(|| CoreError::invalid("Recebimento não encontrado."))?;
    let mut q=c.prepare("SELECT i.id,i.sku_id,i.supplier_code,coalesce(i.source_description,p.name,''),i.source_barcode,i.quantity,i.units_per_pack,i.units,i.commercial_unit,i.unit_price,i.total_cents,i.unit_cost_cents,i.rounding_difference_cents,i.tax_quantity,i.tax_unit,i.source_tax_barcode,i.source_quantity,i.source_unit_price,i.source_total_cents,i.source_adjustment_cents,i.source_lots FROM receipt_items i LEFT JOIN skus s ON s.id=i.sku_id LEFT JOIN products p ON p.id=s.product_id WHERE i.receipt_id=?1 ORDER BY i.id")?;
    let mut items = q
        .query_map([id], |r| {
            Ok(ReceiptItem {
                id: r.get(0)?,
                sku_id: r.get(1)?,
                supplier_code: r.get(2)?,
                description: r.get(3)?,
                barcode: r.get(4)?,
                quantity: r.get(5)?,
                units_per_pack: r.get(6)?,
                units: r.get(7)?,
                commercial_unit: r.get(8)?,
                unit_price: r.get(9)?,
                total_cents: r.get(10)?,
                unit_cost_cents: r.get(11)?,
                rounding_difference_cents: r.get(12)?,
                tax_quantity: r.get(13)?,
                tax_unit: r.get(14)?,
                tax_barcode: r.get(15)?,
                source_quantity: r.get(16)?,
                source_unit_price: r.get(17)?,
                source_total_cents: r.get(18)?,
                source_adjustment_cents: r.get(19)?,
                source_lots: serde_json::from_str(&r.get::<_, String>(20)?).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        20,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
                lots: vec![],
                candidates: vec![],
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for item in &mut items {
        let mut lots=c.prepare("SELECT label,manufactured_on,expires_on,vintage,quantity FROM receipt_item_lots WHERE item_id=?1 ORDER BY id")?;
        item.lots = lots
            .query_map([item.id], |r| {
                Ok(LotDraft {
                    label: r.get(0)?,
                    manufactured_on: r.get(1)?,
                    expires_on: r.get(2)?,
                    vintage: r.get(3)?,
                    quantity: r.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut candidates = std::collections::BTreeSet::new();
        for code in [&item.barcode, &item.tax_barcode].into_iter().flatten() {
            for sku in resolve(c, code, false)? {
                candidates.insert(sku);
            }
        }
        item.candidates = candidates.into_iter().collect();
    }
    Ok(ReceiptDetail { receipt, items })
}
impl Store {
    pub fn receipts(&self) -> Result<Vec<ReceiptSummary>> {
        let mut q = self
            .db
            .connection
            .prepare(&format!("{SUMMARY} ORDER BY r.id DESC LIMIT 500"))?;
        let rows = q
            .query_map([], summary_row)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
    pub fn receipt(&self, id: i64) -> Result<ReceiptDetail> {
        detail(&self.db.connection, id)
    }
    pub fn create_receipt(&mut self, input: ReceiptCreate) -> Result<ReceiptDetail> {
        let notes = text(&input.notes, "Observação", 1000, false)?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(s) = input.supplier_id {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM suppliers WHERE id=?1 AND active=1)",
                [s],
                |r| r.get(0),
            )?;
            if !exists {
                return Err(CoreError::invalid("Fornecedor inexistente ou inativo."));
            }
        }
        tx.execute(
            "INSERT INTO receipts(supplier_id,notes) VALUES(?1,?2)",
            params![input.supplier_id, notes],
        )?;
        let id = tx.last_insert_rowid();
        audit(&tx, "receipt_created", id, "Rascunho manual")?;
        tx.commit()?;
        self.receipt(id)
    }
    pub fn save_receipt_item(&mut self, input: ReceiptItemInput) -> Result<ReceiptDetail> {
        let units = Decimal::parse(&input.quantity)?.units(input.units_per_pack)?;
        let commercial_unit = text(&input.commercial_unit, "Unidade comercial", 20, true)?;
        Decimal::parse(&input.unit_price)?;
        cents(input.total_cents)?;
        let (cost, difference) = unit_cost(input.total_cents, units)?;
        let mut allocated = 0_i64;
        if input.lots.len() > 500 {
            return Err(CoreError::invalid("Limite de 500 lotes por linha."));
        }
        for lot in &input.lots {
            validate_lot(lot)?;
            allocated = allocated
                .checked_add(lot.quantity)
                .ok_or_else(|| CoreError::invalid("Quantidade de lotes excede o limite."))?;
        }
        if !input.lots.is_empty() && allocated != units {
            return Err(CoreError::invalid(
                "A soma dos lotes deve ser igual à quantidade recebida.",
            ));
        }
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        draft(&tx, input.receipt_id, input.expected_revision)?;
        if let Some(sku) = input.sku_id {
            active_sku(&tx, sku)?;
        }
        let id = if let Some(id) = input.item_id {
            if tx.execute("UPDATE receipt_items SET sku_id=?1,quantity=?2,units_per_pack=?3,units=?4,commercial_unit=?5,unit_price=?6,total_cents=?7,unit_cost_cents=?8,rounding_difference_cents=?9 WHERE id=?10 AND receipt_id=?11",params![input.sku_id,input.quantity,input.units_per_pack,units,commercial_unit,input.unit_price,input.total_cents,cost,difference,id,input.receipt_id])?!=1 {return Err(CoreError::invalid("Item não pertence ao recebimento."));}
            id
        } else {
            if input.sku_id.is_none() {
                return Err(CoreError::invalid("Selecione o SKU do recebimento manual."));
            }
            let count: i64 = tx.query_row(
                "SELECT count(*) FROM receipt_items WHERE receipt_id=?1",
                [input.receipt_id],
                |r| r.get(0),
            )?;
            if count >= 5000 {
                return Err(CoreError::invalid("Limite de 5000 itens por recebimento."));
            }
            tx.execute("INSERT INTO receipt_items(receipt_id,sku_id,quantity,units_per_pack,units,commercial_unit,unit_price,total_cents,unit_cost_cents,rounding_difference_cents) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![input.receipt_id,input.sku_id,input.quantity,input.units_per_pack,units,commercial_unit,input.unit_price,input.total_cents,cost,difference])?;
            tx.last_insert_rowid()
        };
        tx.execute("DELETE FROM receipt_item_lots WHERE item_id=?1", [id])?;
        for lot in &input.lots {
            tx.execute("INSERT INTO receipt_item_lots(item_id,label,manufactured_on,expires_on,vintage,quantity) VALUES(?1,?2,?3,?4,?5,?6)",params![id,lot.label.trim(),lot.manufactured_on,lot.expires_on,lot.vintage,lot.quantity])?;
        }
        if input.remember_mapping {
            let supplier: Option<i64> = tx.query_row(
                "SELECT supplier_id FROM receipts WHERE id=?1",
                [input.receipt_id],
                |r| r.get(0),
            )?;
            let code: Option<String> = tx.query_row(
                "SELECT supplier_code FROM receipt_items WHERE id=?1",
                [id],
                |r| r.get(0),
            )?;
            if let (Some(supplier), Some(code), Some(sku)) = (supplier, code, input.sku_id) {
                tx.execute("INSERT INTO supplier_product_mappings(supplier_id,supplier_code,sku_id,units_per_pack) VALUES(?1,?2,?3,?4) ON CONFLICT(supplier_id,supplier_code) DO UPDATE SET sku_id=excluded.sku_id,units_per_pack=excluded.units_per_pack",params![supplier,code,sku,input.units_per_pack])?;
                audit(
                    &tx,
                    "supplier_mapping_confirmed",
                    id,
                    "Associação e conversão confirmadas pelo operador",
                )?;
            }
        }
        let total: i64 = tx.query_row(
            "SELECT coalesce(sum(total_cents),0) FROM receipt_items WHERE receipt_id=?1",
            [input.receipt_id],
            |r| r.get(0),
        )?;
        cents(total)?;
        tx.execute(
            "UPDATE receipts SET revision=revision+1 WHERE id=?1",
            [input.receipt_id],
        )?;
        tx.commit()?;
        self.receipt(input.receipt_id)
    }
    pub fn delete_receipt_item(&mut self, input: DeleteReceiptItem) -> Result<ReceiptDetail> {
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        draft(&tx, input.receipt_id, input.expected_revision)?;
        tx.execute(
            "DELETE FROM receipt_items WHERE receipt_id=?1 AND id=?2",
            params![input.receipt_id, input.item_id],
        )?;
        tx.execute(
            "UPDATE receipts SET revision=revision+1 WHERE id=?1",
            [input.receipt_id],
        )?;
        tx.commit()?;
        self.receipt(input.receipt_id)
    }
    pub fn confirm_receipt(&mut self, input: ReceiptAction) -> Result<Confirmation> {
        if !input.confirmed {
            return Err(CoreError::invalid("Confirme a conferência do recebimento."));
        }
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let d = detail(&tx, input.id)?;
        if d.receipt.status == "confirmed" {
            return Ok(Confirmation {
                id: input.id,
                already_completed: true,
            });
        }
        draft(&tx, input.id, input.expected_revision)?;
        if d.items.is_empty() {
            return Err(CoreError::invalid(
                "Inclua ao menos um item no recebimento.",
            ));
        }
        let mut line = 0_i64;
        let mut costs = std::collections::BTreeMap::<i64, (i128, i128)>::new();
        for item in &d.items {
            let sku = item.sku_id.ok_or_else(|| {
                CoreError::invalid("Associe todos os itens a um SKU antes de confirmar.")
            })?;
            active_sku(&tx, sku)?;
            assert_unlocked(&tx, sku)?;
            let units = Decimal::parse(&item.quantity)?.units(item.units_per_pack)?;
            if Some(units) != item.units {
                return Err(CoreError::invalid("Quantidade convertida inconsistente."));
            }
            let item_cost = item.unit_cost_cents.ok_or_else(|| {
                CoreError::invalid("Confira o custo convertido antes de confirmar.")
            })?;
            let sums = costs.entry(sku).or_default();
            sums.0 += item.total_cents as i128;
            sums.1 += units as i128;
            if item.lots.is_empty() {
                movement(
                    &tx,
                    crate::stock::StockWrite {
                        sku,
                        lot: None,
                        quantity: units,
                        kind: "receipt",
                        origin_type: "receipt",
                        origin_id: &input.id.to_string(),
                        origin_line: line,
                        reason: "Recebimento confirmado",
                        unit_cost_cents: item_cost,
                    },
                )?;
                line += 1;
            } else {
                let total: i64 = item.lots.iter().map(|l| l.quantity).sum();
                if total != units {
                    return Err(CoreError::invalid("Distribuição de lotes incompleta."));
                }
                for lot in &item.lots {
                    validate_lot(lot)?;
                    tx.execute("INSERT INTO lots(sku_id,label,manufactured_on,expires_on,vintage,supplier_id,receipt_id,receipt_item_id,received_quantity,unit_cost_cents) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![sku,lot.label,lot.manufactured_on,lot.expires_on,lot.vintage,d.receipt.supplier_id,input.id,item.id,lot.quantity,item_cost])?;
                    let lot_id = tx.last_insert_rowid();
                    movement(
                        &tx,
                        crate::stock::StockWrite {
                            sku,
                            lot: Some(lot_id),
                            quantity: lot.quantity,
                            kind: "receipt",
                            origin_type: "receipt",
                            origin_id: &input.id.to_string(),
                            origin_line: line,
                            reason: "Recebimento confirmado",
                            unit_cost_cents: item_cost,
                        },
                    )?;
                    line += 1;
                }
            }
        }
        for (sku, (total, units)) in costs {
            let cost = i64::try_from((total + units / 2) / units)
                .map_err(|_| CoreError::invalid("Custo excede o limite."))?;
            cents(cost)?;
            tx.execute(
                "UPDATE skus SET purchase_cents=?1 WHERE id=?2",
                params![cost, sku],
            )?;
        }
        tx.execute("UPDATE receipts SET status='confirmed',revision=revision+1,confirmed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",[input.id])?;
        audit(
            &tx,
            "receipt_confirmed",
            input.id,
            "Conferência explícita; entrada atômica",
        )?;
        tx.commit()?;
        Ok(Confirmation {
            id: input.id,
            already_completed: false,
        })
    }
    pub fn cancel_receipt(&mut self, input: ReceiptAction) -> Result<()> {
        if !input.confirmed {
            return Err(CoreError::invalid("Confirme o cancelamento."));
        }
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        draft(&tx, input.id, input.expected_revision)?;
        tx.execute(
            "UPDATE receipts SET status='canceled',revision=revision+1 WHERE id=?1",
            [input.id],
        )?;
        audit(
            &tx,
            "receipt_canceled",
            input.id,
            "Cancelamento de rascunho sem alterar estoque",
        )?;
        tx.commit()?;
        Ok(())
    }
}
