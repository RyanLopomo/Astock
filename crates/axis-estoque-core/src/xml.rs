//! Importação local de campos NF-e 4.00. Não verifica assinatura/autorização fiscal.
use crate::{
    catalog::resolve,
    decimal::{unit_cost, Decimal},
    model::*,
    stock::audit,
    storage::Store,
};
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_XML_BYTES: usize = 8 * 1024 * 1024;
const NS: &str = "http://www.portalfiscal.inf.br/nfe";
#[derive(Default)]
struct Node {
    name: String,
    nfe: bool,
    attributes: BTreeMap<String, String>,
    text: String,
    children: Vec<Node>,
}
impl Node {
    fn children<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a Node> + 'a {
        let name = name.to_owned();
        self.children
            .iter()
            .filter(move |n| n.nfe && n.name == name)
    }
    fn optional(&self, name: &str) -> Result<Option<&Node>> {
        let mut found = self.children(name);
        let first = found.next();
        if found.next().is_some() {
            return Err(CoreError::invalid(format!(
                "XML contém campo {name} duplicado."
            )));
        }
        Ok(first)
    }
    fn one(&self, name: &str) -> Result<&Node> {
        self.optional(name)?
            .ok_or_else(|| CoreError::invalid(format!("XML sem o campo obrigatório {name}.")))
    }
    fn value(&self, name: &str) -> Result<String> {
        text(&self.one(name)?.text, name, 4096, true)
    }
    fn optional_value(&self, name: &str) -> Result<Option<String>> {
        self.optional(name)?
            .map(|n| text(&n.text, name, 4096, false))
            .transpose()
    }
}
#[derive(Debug)]
pub struct PurchaseXml {
    pub access_key: String,
    pub number: String,
    pub series: String,
    pub issued_at: String,
    pub supplier_name: String,
    pub supplier_document: String,
    pub items: Vec<XmlItem>,
    pub warnings: Vec<String>,
}
#[derive(Debug)]
pub struct XmlItem {
    pub supplier_code: String,
    pub description: String,
    pub barcode: Option<String>,
    pub tax_barcode: Option<String>,
    pub quantity: String,
    pub unit: String,
    pub unit_price: String,
    pub source_total_cents: i64,
    pub adjustment_cents: i64,
    pub total_cents: i64,
    pub tax_quantity: Option<String>,
    pub tax_unit: Option<String>,
    pub lots: Vec<XmlLot>,
}
fn malformed() -> CoreError {
    CoreError::invalid(
        "XML malformado. Selecione uma NF-e 4.00 em UTF-8, sem DTD ou entidades externas.",
    )
}
fn parse_tree(content: &str) -> Result<Node> {
    if content.len() > MAX_XML_BYTES {
        return Err(CoreError::invalid("O XML excede o limite de 8 MiB."));
    }
    let mut reader = NsReader::from_str(content.trim_start_matches('\u{feff}'));
    reader.config_mut().expand_empty_elements = true;
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<Node> = vec![];
    let mut root = None;
    let mut events = 0;
    loop {
        events += 1;
        if events > 200_000 {
            return Err(CoreError::invalid("XML excede o limite de processamento."));
        }
        let (namespace, event) = reader.read_resolved_event().map_err(|_| malformed())?;
        match event {
            Event::Start(e) => {
                if stack.len() >= 32 {
                    return Err(CoreError::invalid("XML excede a profundidade permitida."));
                }
                let nfe = matches!(namespace,ResolveResult::Bound(n) if n.as_ref()==NS);
                let mut attributes = BTreeMap::new();
                for attr in e.attributes() {
                    let a = attr.map_err(|_| malformed())?;
                    if attributes.len() >= 100 {
                        return Err(malformed());
                    }
                    attributes.insert(
                        a.key.as_ref().to_string(),
                        a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map_err(|_| malformed())?
                            .into_owned(),
                    );
                }
                stack.push(Node {
                    name: e.local_name().as_ref().to_string(),
                    nfe,
                    attributes,
                    ..Node::default()
                });
            }
            Event::End(_) => {
                let node = stack.pop().ok_or_else(malformed)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else if root.replace(node).is_some() {
                    return Err(malformed());
                }
            }
            Event::Text(e) => append(&mut stack, e.as_ref())?,
            Event::CData(e) => append(&mut stack, e.as_ref())?,
            Event::GeneralRef(e) => {
                let value = if let Some(c) = e.resolve_char_ref().map_err(|_| malformed())? {
                    c.to_string()
                } else {
                    match e.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => return Err(CoreError::invalid("Entidade XML não permitida.")),
                    }
                    .into()
                };
                append(&mut stack, &value)?;
            }
            Event::DocType(_) => {
                return Err(CoreError::invalid(
                    "DTD e entidades externas não são permitidos.",
                ))
            }
            Event::Decl(e) => {
                if e.encoding().is_some_and(|encoding| {
                    encoding.map_or(true, |v| !v.eq_ignore_ascii_case("UTF-8"))
                }) {
                    return Err(CoreError::invalid("Importação aceita XML em UTF-8."));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(malformed());
    }
    root.ok_or_else(malformed)
}
fn append(stack: &mut [Node], value: &str) -> Result<()> {
    if value
        .chars()
        .any(|c| c.is_control() && !['\n', '\r', '\t'].contains(&c))
    {
        return Err(malformed());
    }
    if let Some(node) = stack.last_mut() {
        if node.text.len() + value.len() > 8192 {
            return Err(CoreError::invalid("Campo XML excede o limite."));
        }
        node.text.push_str(value);
    } else if !value.trim().is_empty() {
        return Err(malformed());
    }
    Ok(())
}
pub fn validate_access_key(key: &str) -> Result<()> {
    if key.len() != 44 || !key.bytes().all(|c| c.is_ascii_digit()) {
        return Err(CoreError::invalid(
            "Chave de acesso deve conter exatamente 44 dígitos.",
        ));
    }
    let uf = &key[..2];
    if ![
        "11", "12", "13", "14", "15", "16", "17", "21", "22", "23", "24", "25", "26", "27", "28",
        "29", "31", "32", "33", "35", "41", "42", "43", "50", "51", "52", "53",
    ]
    .contains(&uf)
    {
        return Err(CoreError::invalid("UF inválida na chave de acesso."));
    }
    if &key[20..22] != "55" || !(1..=12).contains(&key[4..6].parse::<i32>().unwrap_or(0)) {
        return Err(CoreError::invalid(
            "Chave não corresponde ao modelo NF-e 55 ou possui mês inválido.",
        ));
    }
    let mut weight = 2;
    let mut sum = 0;
    for b in key.as_bytes()[..43].iter().rev() {
        sum += (b - b'0') as i32 * weight;
        weight = if weight == 9 { 2 } else { weight + 1 };
    }
    let remainder = sum % 11;
    let digit = if remainder < 2 { 0 } else { 11 - remainder };
    if (key.as_bytes()[43] - b'0') as i32 != digit {
        return Err(CoreError::invalid(
            "Dígito verificador da chave de acesso inválido.",
        ));
    }
    Ok(())
}
fn barcode(value: Option<String>) -> Option<String> {
    value.filter(|v| !v.is_empty() && !v.eq_ignore_ascii_case("SEM GTIN"))
}
pub fn parse_purchase_xml(content: &str) -> Result<PurchaseXml> {
    let root = parse_tree(content)?;
    if !root.nfe {
        return Err(CoreError::invalid("Namespace NF-e não suportado."));
    }
    let nfe = match root.name.as_str() {
        "NFe" => &root,
        "nfeProc" | "procNFe" => root.one("NFe")?,
        _ => {
            return Err(CoreError::invalid(
                "Estrutura não suportada. Use NFe ou nfeProc/procNFe, versão 4.00.",
            ))
        }
    };
    let info = nfe.one("infNFe")?;
    if info.attributes.get("versao").map(String::as_str) != Some("4.00") {
        return Err(CoreError::invalid(
            "Somente o leiaute NF-e 4.00 é suportado.",
        ));
    }
    let key = info
        .attributes
        .get("Id")
        .and_then(|id| id.strip_prefix("NFe"))
        .ok_or_else(|| CoreError::invalid("XML sem identificador NFe/chave de acesso."))?
        .to_string();
    validate_access_key(&key)?;
    let ide = info.one("ide")?;
    if ide.value("mod")? != "55" {
        return Err(CoreError::invalid(
            "Somente NF-e modelo 55 é suportada; NFC-e não é importada.",
        ));
    }
    let number = ide.value("nNF")?;
    let series = ide.value("serie")?;
    if number.parse::<u32>().ok() != key[25..34].parse::<u32>().ok()
        || series.parse::<u32>().ok() != key[22..25].parse::<u32>().ok()
    {
        return Err(CoreError::invalid(
            "Número/série não correspondem à chave de acesso.",
        ));
    }
    let issued_at = ide.value("dhEmi")?;
    let emitted = chrono::DateTime::parse_from_rfc3339(&issued_at)
        .map_err(|_| CoreError::invalid("Data de emissão inválida."))?;
    if emitted.format("%y%m").to_string() != key[2..6] {
        return Err(CoreError::invalid(
            "Data de emissão não corresponde à chave de acesso.",
        ));
    }
    if ide.optional_value("cUF")?.is_some_and(|uf| uf != key[..2]) {
        return Err(CoreError::invalid("UF não corresponde à chave de acesso."));
    }
    let emit = info.one("emit")?;
    let supplier_document = emit
        .optional_value("CNPJ")?
        .or(emit.optional_value("CPF")?)
        .ok_or_else(|| CoreError::invalid("XML sem CNPJ/CPF do emitente."))?;
    if ![11, 14].contains(&supplier_document.len())
        || !supplier_document.bytes().all(|b| b.is_ascii_digit())
        || format!("{supplier_document:0>14}") != key[6..20]
    {
        return Err(CoreError::invalid(
            "Documento do emitente não corresponde à chave de acesso.",
        ));
    }
    let supplier_name = text(&emit.value("xNome")?, "Nome do fornecedor", 160, true)?;
    if let Some(protocol) = root.optional("protNFe")? {
        if protocol.one("infProt")?.value("chNFe")? != key {
            return Err(CoreError::invalid(
                "A chave do protocolo difere da chave da NF-e.",
            ));
        }
    }
    let mut items = vec![];
    let mut numbers = BTreeSet::new();
    let mut warnings = vec![
        "Leitura local: assinatura, autenticidade e autorização fiscal não verificadas.".into(),
    ];
    for det in info.children("det") {
        if items.len() >= 5000 {
            return Err(CoreError::invalid("Limite de 5000 itens por XML."));
        }
        let item_number = det
            .attributes
            .get("nItem")
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|n| *n > 0)
            .ok_or_else(|| CoreError::invalid("Item XML sem nItem válido."))?;
        if !numbers.insert(item_number) {
            return Err(CoreError::invalid("nItem duplicado no XML."));
        }
        let prod = det.one("prod")?;
        let quantity = prod.value("qCom")?;
        let q = Decimal::parse(&quantity)?;
        if !q.is_positive() {
            return Err(CoreError::invalid(
                "Quantidade comercial deve ser positiva.",
            ));
        }
        let unit_price = prod.value("vUnCom")?;
        let price = Decimal::parse(&unit_price)?;
        let subtotal = Decimal::parse(&prod.value("vProd")?)?.cents()?;
        if q.product_cents(price)? != subtotal {
            warnings.push(format!("Item {item_number}: qCom × vUnCom difere de vProd após arredondamento. Confira o custo."));
        }
        let mut adjustments = 0_i64;
        for (field, sign) in [("vDesc", -1), ("vFrete", 1), ("vSeg", 1), ("vOutro", 1)] {
            if let Some(value) = prod.optional_value(field)? {
                let amount = Decimal::parse(&value)?.cents()?;
                adjustments = adjustments
                    .checked_add(sign * amount)
                    .ok_or_else(|| CoreError::invalid("Ajustes monetários excedem o limite."))?;
            }
        }
        let total = subtotal
            .checked_add(adjustments)
            .ok_or_else(|| CoreError::invalid("Custo excede o limite."))?;
        cents(total)?;
        let mut lots = vec![];
        for lot in prod.children("rastro") {
            if lots.len() >= 500 {
                return Err(CoreError::invalid("Limite de 500 rastros por item."));
            }
            let q_lot = lot.value("qLote")?;
            if !Decimal::parse(&q_lot)?.is_positive() {
                return Err(CoreError::invalid("Quantidade de rastro inválida."));
            }
            let manufactured_on = lot.optional_value("dFab")?;
            let expires_on = lot.optional_value("dVal")?;
            if let Some(d) = &manufactured_on {
                date(d)?;
            }
            if let Some(d) = &expires_on {
                date(d)?;
            }
            if manufactured_on
                .as_ref()
                .zip(expires_on.as_ref())
                .is_some_and(|(a, b)| a > b)
            {
                return Err(CoreError::invalid(
                    "Validade do rastro anterior à fabricação.",
                ));
            }
            lots.push(XmlLot {
                label: text(&lot.value("nLote")?, "Lote XML", 100, true)?,
                manufactured_on,
                expires_on,
                quantity: q_lot,
            });
        }
        let tax_quantity = prod.optional_value("qTrib")?;
        if let Some(value) = &tax_quantity {
            Decimal::parse(value)?;
        }
        items.push(XmlItem {
            supplier_code: text(&prod.value("cProd")?, "Código do fornecedor", 160, true)?,
            description: text(&prod.value("xProd")?, "Descrição XML", 300, true)?,
            barcode: barcode(prod.optional_value("cEAN")?),
            tax_barcode: barcode(prod.optional_value("cEANTrib")?),
            quantity,
            unit: text(&prod.value("uCom")?, "Unidade", 20, true)?,
            unit_price,
            source_total_cents: subtotal,
            adjustment_cents: adjustments,
            total_cents: total,
            tax_quantity,
            tax_unit: prod.optional_value("uTrib")?,
            lots,
        });
    }
    if items.is_empty() {
        return Err(CoreError::invalid("XML sem itens de mercadoria."));
    }
    Ok(PurchaseXml {
        access_key: key,
        number,
        series,
        issued_at,
        supplier_name,
        supplier_document,
        items,
        warnings,
    })
}
impl Store {
    pub fn import_xml(&mut self, input: XmlImportInput) -> Result<XmlImportResult> {
        let parsed = parse_purchase_xml(&input.content)?;
        let file_name = text(&input.file_name, "Nome do arquivo", 255, false)?;
        let tx = self
            .db
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let previous: Option<i64> = tx
            .query_row(
                "SELECT receipt_id FROM purchase_xml_imports WHERE access_key=?1",
                [&parsed.access_key],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = previous {
            return Ok(XmlImportResult{detail:crate::receipts::detail(&tx,id)?,reused:true,warnings:vec!["Esta chave já foi importada. O recebimento existente foi recuperado; nenhum novo estoque foi criado.".into()]});
        }
        let supplier: Option<i64> = tx
            .query_row(
                "SELECT id FROM suppliers WHERE tax_id=?1",
                [&parsed.supplier_document],
                |r| r.get(0),
            )
            .optional()?;
        let supplier = if let Some(id) = supplier {
            id
        } else {
            tx.execute(
                "INSERT INTO suppliers(name,tax_id) VALUES(?1,?2)",
                params![parsed.supplier_name, parsed.supplier_document],
            )?;
            tx.last_insert_rowid()
        };
        tx.execute("INSERT INTO receipts(supplier_id,source,notes) VALUES(?1,'xml','XML importado; associação, conversão e conferência pendentes')",[supplier])?;
        let id = tx.last_insert_rowid();
        tx.execute("INSERT INTO purchase_xml_imports(receipt_id,access_key,invoice_number,invoice_series,issued_at,supplier_name,xml_text,file_name) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![id,parsed.access_key,parsed.number,parsed.series,parsed.issued_at,parsed.supplier_name,input.content,file_name])?;
        for item in &parsed.items {
            let mapping:Option<(i64,i64)>=tx.query_row("SELECT m.sku_id,m.units_per_pack FROM supplier_product_mappings m JOIN skus s ON s.id=m.sku_id JOIN products p ON p.id=s.product_id WHERE m.supplier_id=?1 AND m.supplier_code=?2 AND s.active=1 AND p.active=1",params![supplier,item.supplier_code],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
            let mut candidates = BTreeSet::new();
            for barcode in [&item.barcode, &item.tax_barcode].into_iter().flatten() {
                candidates.extend(resolve(&tx, barcode, false)?);
            }
            let (sku, pack) = if let Some((sku, pack)) = mapping {
                (Some(sku), pack)
            } else {
                (
                    if candidates.len() == 1 {
                        candidates.first().copied()
                    } else {
                        None
                    },
                    1,
                )
            };
            let units = Decimal::parse(&item.quantity)?.units(pack).ok();
            let cost = units.map(|u| unit_cost(item.total_cents, u)).transpose()?;
            tx.execute("INSERT INTO receipt_items(receipt_id,sku_id,quantity,units_per_pack,units,commercial_unit,unit_price,total_cents,unit_cost_cents,rounding_difference_cents,supplier_code,source_description,source_barcode,tax_quantity,tax_unit,source_tax_barcode,source_total_cents,source_adjustment_cents,source_quantity,source_unit_price,source_lots) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?3,?7,?19)",params![id,sku,item.quantity,pack,units,item.unit,item.unit_price,item.total_cents,cost.map(|c|c.0),cost.map(|c|c.1),item.supplier_code,item.description,item.barcode,item.tax_quantity,item.tax_unit,item.tax_barcode,item.source_total_cents,item.adjustment_cents,serde_json::to_string(&item.lots)?])?;
        }
        audit(
            &tx,
            "xml_imported",
            id,
            "Importação local em rascunho; sem alteração de estoque",
        )?;
        let total: i64 = tx.query_row(
            "SELECT sum(total_cents) FROM receipt_items WHERE receipt_id=?1",
            [id],
            |r| r.get(0),
        )?;
        cents(total)?;
        let d = crate::receipts::detail(&tx, id)?;
        tx.commit()?;
        let mut warnings = parsed.warnings;
        warnings.push("qTrib e qLote não definem automaticamente a conversão física. Confira unidades por embalagem e distribua os lotes antes de confirmar.".into());
        Ok(XmlImportResult {
            detail: d,
            reused: false,
            warnings,
        })
    }
}
