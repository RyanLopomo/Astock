use crate::{model::*, storage::Store};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use std::collections::BTreeSet;

pub(crate) fn audit(c: &Connection, kind: &str, id: i64, reason: &str) -> Result<()> {
    c.execute(
        "INSERT INTO audit_events(kind,entity_id,reason) VALUES(?1,?2,?3)",
        params![kind, id.to_string(), reason],
    )?;
    Ok(())
}
pub(crate) fn assert_unlocked(c: &Connection, sku: i64) -> Result<()> {
    let locked:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM inventory_skus s JOIN inventories i ON i.id=s.inventory_id WHERE s.sku_id=?1 AND i.status IN('active','closing'))",[sku],|r|r.get(0))?;
    if locked {
        return Err(CoreError::new(
            "INVENTORY_LOCK",
            "SKU bloqueado por inventário em andamento. Finalize ou cancele a sessão.",
        ));
    }
    Ok(())
}
pub(crate) fn active_sku(c: &Connection, sku: i64) -> Result<i64> {
    c.query_row("SELECT s.purchase_cents FROM skus s JOIN products p ON p.id=s.product_id WHERE s.id=?1 AND s.active=1 AND p.active=1",[sku],|r|r.get(0)).optional()?.ok_or_else(||CoreError::invalid("SKU inexistente ou inativo."))
}
pub(crate) fn pool_balance(c: &Connection, sku: i64, lot: Option<i64>) -> Result<i64> {
    Ok(c.query_row(
        "SELECT coalesce(sum(quantity),0) FROM stock_movements WHERE sku_id=?1 AND lot_id IS ?2",
        params![sku, lot],
        |r| r.get(0),
    )?)
}
pub(crate) struct StockWrite<'a> {
    pub sku: i64,
    pub lot: Option<i64>,
    pub quantity: i64,
    pub kind: &'a str,
    pub origin_type: &'a str,
    pub origin_id: &'a str,
    pub origin_line: i64,
    pub reason: &'a str,
    pub unit_cost_cents: i64,
}
pub(crate) fn movement(c: &Connection, value: StockWrite<'_>) -> Result<()> {
    let StockWrite {
        sku,
        lot,
        quantity: q,
        kind,
        origin_type: origin,
        origin_id: id,
        origin_line: line,
        reason,
        unit_cost_cents: cost,
    } = value;
    let cost = if let Some(lot_id) = lot {
        c.query_row(
            "SELECT unit_cost_cents FROM lots WHERE id=?1 AND sku_id=?2",
            params![lot_id, sku],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| CoreError::invalid("O lote não pertence ao SKU."))?
    } else {
        cost
    };
    quantity(q, false)?;
    if q == 0 {
        return Err(CoreError::invalid(
            "Movimento deve ter quantidade diferente de zero.",
        ));
    }
    cents(cost)?;
    c.execute("INSERT INTO stock_movements(sku_id,lot_id,quantity,kind,origin_type,origin_id,origin_line,reason,unit_cost_cents) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![sku,lot,q,kind,origin,id,line,reason,cost])?;
    Ok(())
}
pub(crate) fn replayed(
    c: &Connection,
    key: &str,
    kind: &str,
    input: &impl Serialize,
) -> Result<bool> {
    text(key, "Identificador da operação", 100, true)?;
    let payload = serde_json::to_string(input)?;
    let existing: Option<(String, String)> = c
        .query_row(
            "SELECT kind,payload FROM operation_requests WHERE request_key=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((old_kind, old_payload)) = existing {
        if old_kind != kind || old_payload != payload {
            return Err(CoreError::new(
                "IDEMPOTENCY",
                "O identificador já foi usado com outros dados.",
            ));
        }
        return Ok(true);
    }
    c.execute(
        "INSERT INTO operation_requests VALUES(?1,?2,?3)",
        params![key, kind, payload],
    )?;
    Ok(false)
}
impl Store {
    pub fn adjust_stock(&mut self, input: AdjustmentInput) -> Result<()> {
        if !input.confirmed {
            return Err(CoreError::invalid("Confirme o ajuste de estoque."));
        }
        quantity(input.quantity, false)?;
        let reason = text(&input.reason, "Motivo do ajuste", 500, true)?;
        let reference = text(&input.reference, "Referência", 200, false)?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if replayed(&tx, &input.request_key, "adjustment", &input)? {
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
        tx.execute("INSERT INTO stock_operations(request_key,kind,reason,reference) VALUES(?1,'adjustment',?2,?3)",params![input.request_key,reason,reference])?;
        movement(
            &tx,
            crate::stock::StockWrite {
                sku: input.sku_id,
                lot: input.lot_id,
                quantity: input.quantity,
                kind: "adjustment",
                origin_type: "adjustment",
                origin_id: &input.request_key,
                origin_line: 0,
                reason: &reason,
                unit_cost_cents: cost,
            },
        )?;
        audit(&tx, "stock_adjustment", input.sku_id, &reason)?;
        tx.commit()?;
        Ok(())
    }
    pub fn issue_stock(&mut self, input: IssueInput) -> Result<()> {
        if !input.confirmed || input.allocations.is_empty() || input.allocations.len() > 5000 {
            return Err(CoreError::invalid("Confira e confirme a saída."));
        }
        let reason = text(&input.reason, "Motivo da saída", 500, true)?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if replayed(&tx, &input.request_key, "issue", &input)? {
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
        let mut pools = BTreeSet::new();
        let today = local_date();
        let mut total = 0_i64;
        for a in &input.allocations {
            quantity(a.quantity, true)?;
            total = total
                .checked_add(a.quantity)
                .ok_or_else(|| CoreError::invalid("Quantidade excede o limite."))?;
            if !pools.insert(a.lot_id) {
                return Err(CoreError::invalid("Lote repetido no plano de saída."));
            }
            if let Some(lot) = a.lot_id {
                let expiry: Option<String> = tx
                    .query_row(
                        "SELECT expires_on FROM lots WHERE id=?1 AND sku_id=?2",
                        params![lot, input.sku_id],
                        |r| r.get(0),
                    )
                    .optional()?
                    .ok_or_else(|| CoreError::invalid("Lote não encontrado para este SKU."))?;
                if expiry.as_ref().is_some_and(|e| e < &today) && !input.allow_expired {
                    return Err(CoreError::invalid(
                        "A saída inclui lote vencido. Exija confirmação explícita para prosseguir.",
                    ));
                }
            }
        }
        quantity(total, true)?;
        tx.execute(
            "INSERT INTO stock_operations(request_key,kind,reason) VALUES(?1,'issue',?2)",
            params![input.request_key, reason],
        )?;
        for (index, a) in input.allocations.iter().enumerate() {
            movement(
                &tx,
                crate::stock::StockWrite {
                    sku: input.sku_id,
                    lot: a.lot_id,
                    quantity: -a.quantity,
                    kind: "issue",
                    origin_type: "issue",
                    origin_id: &input.request_key,
                    origin_line: index as i64,
                    reason: &reason,
                    unit_cost_cents: cost,
                },
            )?;
        }
        audit(&tx, "stock_issue", input.sku_id, &reason)?;
        tx.commit()?;
        Ok(())
    }
    pub fn movements(&self, input: MovementQuery) -> Result<MovementPage> {
        if input.page < 0 || input.page > 1_000_000 {
            return Err(CoreError::invalid("Página inválida."));
        }
        if let Some(d) = &input.from_date {
            date(d)?;
        }
        if let Some(d) = &input.to_date {
            date(d)?;
        }
        if input
            .from_date
            .as_ref()
            .zip(input.to_date.as_ref())
            .is_some_and(|(a, b)| a > b)
        {
            return Err(CoreError::invalid("Intervalo de datas inválido."));
        }
        let search = format!("%{}%", text(&input.search, "Busca", 160, false)?);
        let condition=" FROM stock_movements m JOIN skus s ON s.id=m.sku_id JOIN products p ON p.id=s.product_id LEFT JOIN lots l ON l.id=m.lot_id WHERE (?1='%%' OR p.name LIKE ?1 OR s.internal_code LIKE ?1 OR m.reason LIKE ?1) AND (?2 IS NULL OR m.sku_id=?2) AND (?3 IS NULL OR date(m.created_at,'localtime')>=?3) AND (?4 IS NULL OR date(m.created_at,'localtime')<=?4)";
        let c = &self.db.connection;
        let total = c.query_row(
            &format!("SELECT count(*){condition}"),
            params![search, input.sku_id, input.from_date, input.to_date],
            |r| r.get(0),
        )?;
        let mut q=c.prepare(&format!("SELECT m.id,m.sku_id,s.internal_code,p.name,m.lot_id,coalesce(l.label,'Sem lote identificado'),m.quantity,m.kind,m.origin_type,m.origin_id,m.reason,m.unit_cost_cents,m.created_at{condition} ORDER BY m.id DESC LIMIT 50 OFFSET ?5"))?;
        let items = q
            .query_map(
                params![
                    search,
                    input.sku_id,
                    input.from_date,
                    input.to_date,
                    input.page * 50
                ],
                |r| {
                    Ok(Movement {
                        id: r.get(0)?,
                        sku_id: r.get(1)?,
                        internal_code: r.get(2)?,
                        sku_name: r.get(3)?,
                        lot_id: r.get(4)?,
                        lot_label: r.get(5)?,
                        quantity: r.get(6)?,
                        kind: r.get(7)?,
                        origin_type: r.get(8)?,
                        origin_id: r.get(9)?,
                        reason: r.get(10)?,
                        unit_cost_cents: r.get(11)?,
                        created_at: r.get(12)?,
                    })
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(MovementPage {
            items,
            total,
            page: input.page,
        })
    }
}
