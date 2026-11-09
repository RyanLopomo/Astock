use crate::{
    catalog::resolve,
    model::*,
    stock::{audit, movement, pool_balance, replayed},
    storage::Store,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::collections::BTreeSet;

const HEAD:&str="SELECT i.id,i.name,i.status,i.scope,i.revision,i.created_at,i.closed_at,i.uncounted_decision,(SELECT count(*) FROM inventory_skus WHERE inventory_id=i.id) FROM inventories i";
fn header(r: &rusqlite::Row<'_>) -> rusqlite::Result<InventorySummary> {
    Ok(InventorySummary {
        id: r.get(0)?,
        name: r.get(1)?,
        status: r.get(2)?,
        scope: r.get(3)?,
        revision: r.get(4)?,
        created_at: r.get(5)?,
        closed_at: r.get(6)?,
        uncounted_decision: r.get(7)?,
        sku_count: r.get(8)?,
    })
}
fn active(c: &Connection, id: i64) -> Result<()> {
    let status: Option<String> = c
        .query_row("SELECT status FROM inventories WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    if status.as_deref() != Some("active") {
        return Err(CoreError::invalid(
            "Inventário inexistente ou já encerrado.",
        ));
    }
    Ok(())
}
fn events(c: &Connection, id: i64, page: i64) -> Result<CountEventPage> {
    if !(0..=1_000_000).contains(&page) {
        return Err(CoreError::invalid("Página inválida."));
    }
    let total = c.query_row(
        "SELECT count(*) FROM inventory_events WHERE inventory_id=?1",
        [id],
        |r| r.get(0),
    )?;
    let mut q=c.prepare("SELECT id,code,sku_id,lot_id,quantity,mode,outcome,created_at,previous_count,new_count FROM inventory_events WHERE inventory_id=?1 ORDER BY id DESC LIMIT 50 OFFSET ?2")?;
    let items = q
        .query_map(params![id, page * 50], |r| {
            Ok(CountEvent {
                id: r.get(0)?,
                code: r.get(1)?,
                sku_id: r.get(2)?,
                lot_id: r.get(3)?,
                quantity: r.get(4)?,
                mode: r.get(5)?,
                outcome: r.get(6)?,
                created_at: r.get(7)?,
                previous_count: r.get(8)?,
                new_count: r.get(9)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(CountEventPage { items, total, page })
}
fn detail(c: &Connection, id: i64) -> Result<InventoryDetail> {
    let inventory = c
        .query_row(&format!("{HEAD} WHERE i.id=?1"), [id], header)
        .optional()?
        .ok_or_else(|| CoreError::invalid("Inventário não encontrado."))?;
    let mut q=c.prepare("SELECT n.id,n.sku_id,p.name,s.internal_code,n.lot_id,coalesce(l.label,'Sem lote identificado'),l.expires_on,n.reference,n.counted,CASE WHEN n.counted IS NOT NULL THEN n.counted-n.reference WHEN closed.status='finalized' AND closed.uncounted_decision='zero' THEN -n.reference ELSE NULL END FROM inventory_lines n JOIN inventories closed ON closed.id=n.inventory_id JOIN skus s ON s.id=n.sku_id JOIN products p ON p.id=s.product_id LEFT JOIN lots l ON l.id=n.lot_id WHERE n.inventory_id=?1 ORDER BY p.name,n.sku_id,l.expires_on IS NULL,l.expires_on,n.id")?;
    let lines = q
        .query_map([id], |r| {
            Ok(InventoryLine {
                id: r.get(0)?,
                sku_id: r.get(1)?,
                sku_name: r.get(2)?,
                internal_code: r.get(3)?,
                lot_id: r.get(4)?,
                lot_label: r.get(5)?,
                expires_on: r.get(6)?,
                reference: r.get(7)?,
                counted: r.get(8)?,
                difference: r.get(9)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let page = events(c, id, 0)?;
    Ok(InventoryDetail {
        inventory,
        lines,
        events: page.items,
        events_total: page.total,
    })
}
fn record_event(
    c: &Connection,
    input: &InventoryScan,
    sku: Option<i64>,
    outcome: &str,
    previous: Option<i64>,
    new: Option<i64>,
) -> Result<()> {
    c.execute("INSERT INTO inventory_events(inventory_id,request_key,code,sku_id,lot_id,quantity,mode,outcome,previous_count,new_count) VALUES(?1,?2,?3,?4,?5,?6,'add',?7,?8,?9)",params![input.inventory_id,input.request_key,input.code,sku,if sku.is_some(){input.lot_id}else{None},input.quantity,outcome,previous,new])?;
    Ok(())
}
impl Store {
    pub fn inventories(&self) -> Result<Vec<InventorySummary>> {
        let mut q = self
            .db
            .connection
            .prepare(&format!("{HEAD} ORDER BY i.id DESC LIMIT 500"))?;
        let rows = q
            .query_map([], header)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
    pub fn inventory(&self, id: i64) -> Result<InventoryDetail> {
        detail(&self.db.connection, id)
    }
    pub fn inventory_events(&self, id: i64, page: i64) -> Result<CountEventPage> {
        events(&self.db.connection, id, page)
    }
    pub fn create_inventory(&mut self, input: InventoryCreate) -> Result<InventoryDetail> {
        if input.sku_ids.as_ref().is_some_and(|ids| ids.len() > 10_000) {
            return Err(CoreError::invalid(
                "Limite de 10000 SKUs por seleção parcial.",
            ));
        }
        let name = text(&input.name, "Nome do inventário", 160, true)?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM inventories WHERE status IN('active','closing'))",
            [],
            |r| r.get(0),
        )?;
        if exists {
            return Err(CoreError::new(
                "ACTIVE_INVENTORY",
                "Já existe um inventário em andamento. Retome ou cancele a sessão atual.",
            ));
        }
        let scope = if input.sku_ids.is_some() {
            "partial"
        } else {
            "full"
        };
        let sku_ids = if let Some(ids) = input.sku_ids {
            ids.into_iter().collect::<BTreeSet<_>>()
        } else {
            let mut q=tx.prepare("SELECT s.id FROM skus s JOIN products p ON p.id=s.product_id WHERE (s.active=1 AND p.active=1) OR coalesce((SELECT sum(quantity) FROM stock_movements WHERE sku_id=s.id),0)<>0")?;
            let rows = q
                .query_map([], |r| r.get::<_, i64>(0))?
                .collect::<std::result::Result<BTreeSet<_>, _>>()?;
            rows
        };
        if sku_ids.is_empty() {
            return Err(CoreError::invalid(
                "Selecione ao menos um SKU para o inventário.",
            ));
        }
        tx.execute(
            "INSERT INTO inventories(name,scope) VALUES(?1,?2)",
            params![name, scope],
        )?;
        let id = tx.last_insert_rowid();
        for sku in sku_ids {
            let valid: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM skus WHERE id=?1)",
                [sku],
                |r| r.get(0),
            )?;
            if !valid {
                return Err(CoreError::invalid("Um SKU selecionado não existe."));
            }
            tx.execute("INSERT INTO inventory_skus VALUES(?1,?2)", params![id, sku])?;
            tx.execute("INSERT INTO inventory_lines(inventory_id,sku_id,lot_id,reference) VALUES(?1,?2,NULL,?3)",params![id,sku,pool_balance(&tx,sku,None)?])?;
            let mut q = tx.prepare("SELECT id FROM lots WHERE sku_id=?1 ORDER BY id")?;
            let lots = q
                .query_map([sku], |r| r.get::<_, i64>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            for lot in lots {
                tx.execute("INSERT INTO inventory_lines(inventory_id,sku_id,lot_id,reference) VALUES(?1,?2,?3,?4)",params![id,sku,lot,pool_balance(&tx,sku,Some(lot))?])?;
            }
        }
        audit(
            &tx,
            "inventory_started",
            id,
            "Saldos capturados e SKUs bloqueados",
        )?;
        tx.commit()?;
        self.inventory(id)
    }
    pub fn inventory_scan(&mut self, mut input: InventoryScan) -> Result<ScanResult> {
        input.code = text(&input.code, "Código lido", 2048, true)?;
        quantity(input.quantity, true)?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if replayed(&tx, &input.request_key, "inventory_scan", &input)? {
            let outcome: String = tx.query_row(
                "SELECT outcome FROM inventory_events WHERE request_key=?1",
                [&input.request_key],
                |r| r.get(0),
            )?;
            return Ok(ScanResult {
                outcome,
                message: "Leitura já registrada; quantidade não duplicada.".into(),
                candidates: vec![],
                lot_choices: vec![],
                detail: detail(&tx, input.inventory_id)?,
            });
        }
        active(&tx, input.inventory_id)?;
        let candidates = resolve(&tx, &input.code, true)?;
        let sku = match input.sku_id {
            Some(id) if candidates.contains(&id) => Some(id),
            Some(_) => {
                return Err(CoreError::invalid(
                    "O SKU escolhido não corresponde ao código lido.",
                ))
            }
            None if candidates.len() == 1 => Some(candidates[0]),
            _ => None,
        };
        if sku.is_none() {
            let outcome = if candidates.is_empty() {
                "unknown"
            } else {
                "ambiguous"
            };
            record_event(&tx, &input, None, outcome, None, None)?;
            let d = detail(&tx, input.inventory_id)?;
            tx.commit()?;
            return Ok(ScanResult{outcome:outcome.into(),message:if outcome=="unknown"{"Código não associado. Nenhum produto foi contado. URLs e outros textos não são interpretados como produtos."}else{"Código compartilhado: escolha explicitamente uma variante."}.into(),candidates,lot_choices:vec![],detail:d});
        }
        let sku = sku.unwrap();
        let included: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM inventory_skus WHERE inventory_id=?1 AND sku_id=?2)",
            params![input.inventory_id, sku],
            |r| r.get(0),
        )?;
        if !included {
            return Err(CoreError::invalid(
                "Este SKU não está incluído no inventário. Nenhuma contagem foi alterada.",
            ));
        }
        let d = detail(&tx, input.inventory_id)?;
        let choices = d
            .lines
            .iter()
            .filter(|l| l.sku_id == sku)
            .cloned()
            .collect::<Vec<_>>();
        let tracked: bool =
            tx.query_row("SELECT expiry_control FROM skus WHERE id=?1", [sku], |r| {
                r.get(0)
            })?;
        if !input.lot_selected && (tracked || choices.len() > 1) {
            record_event(&tx, &input, Some(sku), "lot_required", None, None)?;
            let d = detail(&tx, input.inventory_id)?;
            tx.commit()?;
            return Ok(ScanResult {
                outcome: "lot_required".into(),
                message: "Escolha o lote contado ou o saldo sem lote identificado.".into(),
                candidates: vec![sku],
                lot_choices: choices,
                detail: d,
            });
        }
        let line = choices
            .into_iter()
            .find(|l| l.lot_id == input.lot_id)
            .ok_or_else(|| CoreError::invalid("Lote não incluído para este SKU."))?;
        let previous = line.counted.unwrap_or(0);
        let new = previous
            .checked_add(input.quantity)
            .ok_or_else(|| CoreError::invalid("Quantidade excede o limite."))?;
        quantity(new, false)?;
        tx.execute(
            "UPDATE inventory_lines SET counted=?1 WHERE id=?2",
            params![new, line.id],
        )?;
        record_event(&tx, &input, Some(sku), "counted", Some(previous), Some(new))?;
        tx.execute(
            "UPDATE inventories SET revision=revision+1 WHERE id=?1",
            [input.inventory_id],
        )?;
        let d = detail(&tx, input.inventory_id)?;
        tx.commit()?;
        Ok(ScanResult {
            outcome: "counted".into(),
            message: format!(
                "{}: {} unidade(s) contada(s) neste lote.",
                line.sku_name, new
            ),
            candidates: vec![sku],
            lot_choices: vec![],
            detail: d,
        })
    }
    pub fn correct_count(&mut self, input: CountCorrection) -> Result<InventoryDetail> {
        quantity(input.quantity, false)?;
        if input.quantity < 0 {
            return Err(CoreError::invalid("Contagem não pode ser negativa."));
        }
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if replayed(&tx, &input.request_key, "inventory_correction", &input)? {
            return detail(&tx, input.inventory_id);
        }
        active(&tx, input.inventory_id)?;
        let d = detail(&tx, input.inventory_id)?;
        if d.inventory.revision != input.expected_revision {
            return Err(CoreError::new(
                "STALE_REVISION",
                "A contagem foi alterada. Atualize antes de corrigir.",
            ));
        }
        let line = d
            .lines
            .iter()
            .find(|l| l.id == input.line_id)
            .ok_or_else(|| CoreError::invalid("Linha de contagem não encontrada."))?;
        tx.execute(
            "UPDATE inventory_lines SET counted=?1 WHERE id=?2",
            params![input.quantity, input.line_id],
        )?;
        tx.execute("INSERT INTO inventory_events(inventory_id,request_key,code,sku_id,lot_id,quantity,mode,outcome,previous_count,new_count) VALUES(?1,?2,?3,?4,?5,?6,'set','corrected',?7,?6)",params![input.inventory_id,input.request_key,line.internal_code,line.sku_id,line.lot_id,input.quantity,line.counted])?;
        tx.execute(
            "UPDATE inventories SET revision=revision+1 WHERE id=?1",
            [input.inventory_id],
        )?;
        let d = detail(&tx, input.inventory_id)?;
        tx.commit()?;
        Ok(d)
    }
    pub fn finish_inventory(&mut self, input: InventoryFinish) -> Result<Confirmation> {
        if !input.confirmed {
            return Err(CoreError::invalid(
                "Confirme a aplicação dos ajustes por inventário.",
            ));
        }
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let d = detail(&tx, input.id)?;
        if d.inventory.status == "finalized" {
            return Ok(Confirmation {
                id: input.id,
                already_completed: true,
            });
        }
        active(&tx, input.id)?;
        if d.inventory.revision != input.expected_revision {
            return Err(CoreError::new(
                "STALE_REVISION",
                "A contagem mudou. Confira o resumo atualizado antes de finalizar.",
            ));
        }
        let has_uncounted = d.lines.iter().any(|l| l.counted.is_none());
        let decision=match input.uncounted_decision.as_deref(){Some("skip")=>"skip",Some("zero")=>"zero",None if !has_uncounted=>"skip",_=>return Err(CoreError::invalid("Decida explicitamente se os itens não contados serão preservados ou contados como zero."))};
        tx.execute(
            "UPDATE inventories SET status='closing' WHERE id=?1",
            [input.id],
        )?;
        for line in d.lines {
            let counted = match line.counted {
                Some(q) => q,
                None if decision == "zero" => 0,
                None => continue,
            };
            let delta = counted - line.reference;
            if delta != 0 {
                let cost: i64 = tx.query_row(
                    "SELECT purchase_cents FROM skus WHERE id=?1",
                    [line.sku_id],
                    |r| r.get(0),
                )?;
                movement(
                    &tx,
                    crate::stock::StockWrite {
                        sku: line.sku_id,
                        lot: line.lot_id,
                        quantity: delta,
                        kind: "inventory",
                        origin_type: "inventory",
                        origin_id: &input.id.to_string(),
                        origin_line: line.id,
                        reason: "Ajuste por inventário",
                        unit_cost_cents: cost,
                    },
                )?;
            }
        }
        tx.execute("UPDATE inventories SET status='finalized',revision=revision+1,uncounted_decision=?1,closed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?2",params![decision,input.id])?;
        audit(
            &tx,
            "inventory_finalized",
            input.id,
            "Ajuste por inventário; decisão explícita para não contados",
        )?;
        tx.commit()?;
        Ok(Confirmation {
            id: input.id,
            already_completed: false,
        })
    }
    pub fn cancel_inventory(&mut self, input: InventoryCancel) -> Result<()> {
        if !input.confirmed {
            return Err(CoreError::invalid("Confirme o cancelamento do inventário."));
        }
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let status: String = tx
            .query_row(
                "SELECT status FROM inventories WHERE id=?1",
                [input.id],
                |r| r.get(0),
            )
            .optional()?
            .ok_or_else(|| CoreError::invalid("Inventário não encontrado."))?;
        if status == "canceled" {
            return Ok(());
        }
        active(&tx, input.id)?;
        tx.execute("UPDATE inventories SET status='canceled',revision=revision+1,closed_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id=?1",[input.id])?;
        audit(
            &tx,
            "inventory_canceled",
            input.id,
            "Contagens preservadas; nenhum ajuste aplicado",
        )?;
        tx.commit()?;
        Ok(())
    }
}
