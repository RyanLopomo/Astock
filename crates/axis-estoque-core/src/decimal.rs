use crate::model::{CoreError, Result, MAX_CENTS, MAX_QUANTITY};

#[derive(Debug, Clone, Copy)]
pub struct Decimal {
    mantissa: i128,
    scale: u32,
}
impl Decimal {
    pub fn is_positive(self) -> bool {
        self.mantissa > 0
    }
    pub fn parse(value: &str) -> Result<Self> {
        let value = value.trim();
        if value.is_empty() || value.len() > 32 {
            return Err(CoreError::invalid("Decimal inválido ou muito longo."));
        }
        let mut parts = value.split('.');
        let whole = parts.next().unwrap_or("");
        let fraction = parts.next().unwrap_or("");
        if parts.next().is_some()
            || whole.is_empty()
            || !whole.bytes().all(|c| c.is_ascii_digit())
            || !fraction.bytes().all(|c| c.is_ascii_digit())
            || fraction.len() > 10
        {
            return Err(CoreError::invalid(
                "Use decimal positivo com ponto e até dez casas decimais.",
            ));
        }
        let digits = format!("{whole}{fraction}");
        let mantissa = digits
            .parse::<i128>()
            .map_err(|_| CoreError::invalid("Decimal excede o limite."))?;
        Ok(Self {
            mantissa,
            scale: fraction.len() as u32,
        })
    }
    pub fn units(self, pack: i64) -> Result<i64> {
        if !(1..=MAX_QUANTITY).contains(&pack) {
            return Err(CoreError::invalid(
                "A conversão deve ser um inteiro positivo.",
            ));
        }
        let value = self
            .mantissa
            .checked_mul(pack as i128)
            .ok_or_else(|| CoreError::invalid("Quantidade excede o limite."))?;
        let divisor = 10_i128.pow(self.scale);
        if value % divisor != 0 || value / divisor <= 0 || value / divisor > MAX_QUANTITY as i128 {
            return Err(CoreError::invalid(
                "A conversão deve resultar em uma quantidade inteira e positiva de unidades.",
            ));
        }
        Ok((value / divisor) as i64)
    }
    pub fn cents(self) -> Result<i64> {
        let numerator = self
            .mantissa
            .checked_mul(100)
            .ok_or_else(|| CoreError::invalid("Valor excede o limite."))?;
        let divisor = 10_i128.pow(self.scale);
        let value = numerator
            .checked_add(divisor / 2)
            .ok_or_else(|| CoreError::invalid("Valor excede o limite."))?
            / divisor;
        if value > MAX_CENTS as i128 {
            return Err(CoreError::invalid("Valor excede o limite."));
        }
        Ok(value as i64)
    }
    pub fn product_cents(self, other: Self) -> Result<i64> {
        let numerator = self
            .mantissa
            .checked_mul(other.mantissa)
            .and_then(|n| n.checked_mul(100))
            .ok_or_else(|| CoreError::invalid("Valor excede o limite."))?;
        let divisor = 10_i128.pow(self.scale + other.scale);
        let value = numerator
            .checked_add(divisor / 2)
            .ok_or_else(|| CoreError::invalid("Valor excede o limite."))?
            / divisor;
        if value > MAX_CENTS as i128 {
            return Err(CoreError::invalid("Valor excede o limite."));
        }
        Ok(value as i64)
    }
}
pub fn unit_cost(total: i64, units: i64) -> Result<(i64, i64)> {
    if total < 0 || units <= 0 {
        return Err(CoreError::invalid("Custo e quantidade inválidos."));
    }
    let unit = (total as i128 + units as i128 / 2) / units as i128;
    let difference = unit * units as i128 - total as i128;
    Ok((
        i64::try_from(unit).map_err(|_| CoreError::invalid("Custo excede o limite."))?,
        i64::try_from(difference)
            .map_err(|_| CoreError::invalid("Arredondamento excede o limite."))?,
    ))
}
