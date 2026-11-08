use crate::{
    model::*,
    stock::{assert_unlocked, audit},
    storage::Store,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use std::collections::BTreeSet;

impl Store {
    pub fn save_product(&mut self, input: ProductInput) -> Result<Sku> {
        let name = text(&input.name, "Nome", 160, true)?;
        let code = text(&input.internal_code, "Código interno", 64, true)?;
        let description = text(&input.description, "Descrição", 1000, false)?;
        let brand = text(&input.brand, "Marca", 100, false)?;
        let category = text(&input.category, "Categoria", 100, false)?;
        let presentation = text(&input.presentation, "Apresentação", 100, false)?;
        cents(input.purchase_cents)?;
        cents(input.sale_cents)?;
        quantity(input.default_pack, true)?;
        if input.barcodes.len() > 100 {
            return Err(CoreError::invalid("Limite de 100 códigos por SKU."));
        }
        let barcodes = input
            .barcodes
            .iter()
            .map(|c| text(c, "Código de barras", 128, true))
            .collect::<Result<BTreeSet<_>>>()?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(id) = input.sku_id {
            assert_unlocked(&tx, id)?;
        }
        let category_id = if category.is_empty() {
            None
        } else {
            tx.execute(
                "INSERT INTO categories(name) VALUES(?1) ON CONFLICT(name) DO NOTHING",
                [&category],
            )?;
            Some(tx.query_row(
                "SELECT id FROM categories WHERE name=?1",
                [&category],
                |r| r.get::<_, i64>(0),
            )?)
        };
        let product_id = if let Some(id) = input.product_id {
            if let Some(sku) = input.sku_id {
                let owner: i64 = tx
                    .query_row("SELECT product_id FROM skus WHERE id=?1", [sku], |r| {
                        r.get(0)
                    })
                    .optional()?
                    .ok_or_else(|| CoreError::invalid("SKU não encontrado."))?;
                if owner != id {
                    return Err(CoreError::invalid(
                        "O SKU não pertence ao produto informado.",
                    ));
                }
            }
            // Alterações compartilhadas também respeitam o bloqueio de outros SKUs do produto.
            let mut q = tx.prepare("SELECT id FROM skus WHERE product_id=?1")?;
            for sku in q.query_map([id], |r| r.get::<_, i64>(0))? {
                assert_unlocked(&tx, sku?)?;
            }
            if tx.execute("UPDATE products SET name=?1,description=?2,brand=?3,category_id=?4,active=?5 WHERE id=?6",params![name,description,brand,category_id,input.product_active,id])?!=1 { return Err(CoreError::invalid("Produto não encontrado.")); }
            id
        } else {
            if input.sku_id.is_some() {
                return Err(CoreError::invalid("Produto obrigatório para editar SKU."));
            }
            tx.execute("INSERT INTO products(name,description,brand,category_id,active) VALUES(?1,?2,?3,?4,?5)",params![name,description,brand,category_id,input.product_active])?;
            tx.last_insert_rowid()
        };
        let sku_id = if let Some(id) = input.sku_id {
            tx.execute("UPDATE skus SET internal_code=?1,presentation=?2,purchase_cents=?3,sale_cents=?4,default_pack=?5,expiry_control=?6,active=?7 WHERE id=?8",params![code,presentation,input.purchase_cents,input.sale_cents,input.default_pack,input.expiry_control,input.active,id])?;
            id
        } else {
            tx.execute("INSERT INTO skus(product_id,internal_code,presentation,purchase_cents,sale_cents,default_pack,expiry_control,active) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![product_id,code,presentation,input.purchase_cents,input.sale_cents,input.default_pack,input.expiry_control,input.active])?;
            tx.last_insert_rowid()
        };
        tx.execute("DELETE FROM barcodes WHERE sku_id=?1", [sku_id])?;
        for barcode in barcodes {
            tx.execute(
                "INSERT INTO barcodes(sku_id,code) VALUES(?1,?2)",
                params![sku_id, barcode],
            )?;
        }
        audit(&tx, "product_saved", sku_id, "Cadastro/alteração explícita")?;
        tx.commit()?;
        self.skus()?
            .into_iter()
            .find(|s| s.id == sku_id)
            .ok_or_else(|| CoreError::invalid("SKU não encontrado após salvar."))
    }
    pub fn skus(&self) -> Result<Vec<Sku>> {
        let c = &self.db.connection;
        let mut barcode_query = c.prepare("SELECT sku_id,code FROM barcodes ORDER BY code")?;
        let mut codes = std::collections::BTreeMap::<i64, Vec<String>>::new();
        for pair in
            barcode_query.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        {
            let (id, code) = pair?;
            codes.entry(id).or_default().push(code);
        }
        let mut q=c.prepare("SELECT s.id,p.id,p.name,p.description,p.brand,coalesce(c.name,''),s.internal_code,s.presentation,s.purchase_cents,s.sale_cents,s.default_pack,s.expiry_control,s.active,p.active,coalesce(b.total,0),coalesce(b.unidentified,0) FROM skus s JOIN products p ON p.id=s.product_id LEFT JOIN categories c ON c.id=p.category_id LEFT JOIN (SELECT sku_id,sum(quantity) total,sum(CASE WHEN lot_id IS NULL THEN quantity ELSE 0 END) unidentified FROM stock_movements GROUP BY sku_id) b ON b.sku_id=s.id ORDER BY p.name,s.internal_code")?;
        let rows = q.query_map([], |r| {
            let id = r.get(0)?;
            Ok(Sku {
                id,
                product_id: r.get(1)?,
                name: r.get(2)?,
                description: r.get(3)?,
                brand: r.get(4)?,
                category: r.get(5)?,
                internal_code: r.get(6)?,
                presentation: r.get(7)?,
                purchase_cents: r.get(8)?,
                sale_cents: r.get(9)?,
                default_pack: r.get(10)?,
                expiry_control: r.get(11)?,
                active: r.get(12)?,
                product_active: r.get(13)?,
                barcodes: codes.remove(&id).unwrap_or_default(),
                balance: r.get(14)?,
                unidentified_balance: r.get(15)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn resolve_code(&self, code: &str) -> Result<Vec<i64>> {
        resolve(&self.db.connection, code, false)
    }
    pub fn save_supplier(&mut self, input: SupplierInput) -> Result<Supplier> {
        let name = text(&input.name, "Fornecedor", 160, true)?;
        let tax_id = text(&input.tax_id, "CNPJ/CPF", 14, false)?;
        if !tax_id.is_empty()
            && (!tax_id.bytes().all(|c| c.is_ascii_digit()) || ![11, 14].contains(&tax_id.len()))
        {
            return Err(CoreError::invalid(
                "Informe CNPJ com 14 ou CPF com 11 dígitos, sem pontuação.",
            ));
        }
        let contact = text(&input.contact, "Contato", 200, false)?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let id = if let Some(id) = input.id {
            if tx.execute(
                "UPDATE suppliers SET name=?1,tax_id=?2,contact=?3,active=?4 WHERE id=?5",
                params![name, tax_id, contact, input.active, id],
            )? != 1
            {
                return Err(CoreError::invalid("Fornecedor não encontrado."));
            }
            id
        } else {
            tx.execute(
                "INSERT INTO suppliers(name,tax_id,contact,active) VALUES(?1,?2,?3,?4)",
                params![name, tax_id, contact, input.active],
            )?;
            tx.last_insert_rowid()
        };
        audit(&tx, "supplier_saved", id, "Cadastro de fornecedor")?;
        tx.commit()?;
        self.suppliers()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| CoreError::invalid("Fornecedor não encontrado."))
    }
    pub fn suppliers(&self) -> Result<Vec<Supplier>> {
        let mut q = self
            .db
            .connection
            .prepare("SELECT id,name,tax_id,contact,active FROM suppliers ORDER BY name")?;
        let rows = q
            .query_map([], |r| {
                Ok(Supplier {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    tax_id: r.get(2)?,
                    contact: r.get(3)?,
                    active: r.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}
pub(crate) fn resolve(
    c: &rusqlite::Connection,
    code: &str,
    include_inactive: bool,
) -> Result<Vec<i64>> {
    let code = text(code, "Código", 2048, true)?;
    let mut q=c.prepare("SELECT DISTINCT s.id FROM skus s JOIN products p ON p.id=s.product_id LEFT JOIN barcodes b ON b.sku_id=s.id WHERE (s.internal_code=?1 OR b.code=?1) AND (?2 OR (s.active=1 AND p.active=1)) ORDER BY s.id")?;
    let rows = q
        .query_map(params![code, include_inactive], |r| r.get(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}
