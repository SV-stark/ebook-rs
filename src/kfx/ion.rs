//! Clean-room Amazon Ion 1.0 binary encoder and decoder for KFX containers.

/// Amazon Ion 1.0 Binary Version Marker (BVM).
pub const ION_BVM: [u8; 4] = [0xE0, 0x01, 0x00, 0xEA];

/// Subset of Amazon Ion 1.0 values used in KFX containers and entities.
#[derive(Debug, Clone, PartialEq)]
pub enum IonValue {
    Null,
    Bool(bool),
    Int(i64),
    Decimal(Vec<u8>),
    Symbol(u64),
    String(String),
    Blob(Vec<u8>),
    List(Vec<IonValue>),
    Struct(Vec<(u64, IonValue)>),
    Annotated(Vec<u64>, Box<IonValue>),
}

impl IonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            IonValue::String(s) => Some(s.as_str()),
            IonValue::Annotated(_, inner) => inner.as_str(),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            IonValue::Int(v) if *v >= 0 => Some(*v as u64),
            IonValue::Symbol(v) => Some(*v),
            IonValue::Annotated(_, inner) => inner.as_u64(),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[IonValue]> {
        match self {
            IonValue::List(items) => Some(items.as_slice()),
            IonValue::Annotated(_, inner) => inner.as_list(),
            _ => None,
        }
    }

    pub fn as_struct(&self) -> Option<&[(u64, IonValue)]> {
        match self {
            IonValue::Struct(fields) => Some(fields.as_slice()),
            IonValue::Annotated(_, inner) => inner.as_struct(),
            _ => None,
        }
    }

    pub fn get_field(&self, field_sym: u64) -> Option<&IonValue> {
        self.as_struct()?
            .iter()
            .find(|(k, _)| *k == field_sym)
            .map(|(_, v)| v)
    }
}

/// Encode a `VarUInt` (7-bit big-endian, `0x80` stop bit on last byte).
pub fn write_var_uint(out: &mut Vec<u8>, mut val: u64) {
    let mut buf = [0u8; 10];
    let mut i = 0;
    loop {
        buf[i] = (val & 0x7F) as u8;
        val >>= 7;
        i += 1;
        if val == 0 {
            break;
        }
    }
    for idx in (0..i).rev() {
        let mut b = buf[idx];
        if idx == 0 {
            b |= 0x80;
        }
        out.push(b);
    }
}

/// Decode a `VarUInt`, returning `(value, bytes_consumed)`.
pub fn read_var_uint(data: &[u8]) -> Result<(u64, usize), String> {
    let mut val: u64 = 0;
    for (idx, &b) in data.iter().enumerate() {
        if idx >= 9 {
            return Err("Ion VarUInt overflow".to_string());
        }
        val = (val << 7) | u64::from(b & 0x7F);
        if (b & 0x80) != 0 {
            return Ok((val, idx + 1));
        }
    }
    Err("Truncated Ion VarUInt".to_string())
}

fn write_uint_be(out: &mut Vec<u8>, val: u64) -> usize {
    if val == 0 {
        return 0;
    }
    let bytes = val.to_be_bytes();
    let first = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    let slice = &bytes[first..];
    out.extend_from_slice(slice);
    slice.len()
}

fn write_typed_header(out: &mut Vec<u8>, type_code: u8, len: usize) {
    if len < 14 {
        out.push((type_code << 4) | (len as u8));
    } else {
        out.push((type_code << 4) | 0x0E);
        write_var_uint(out, len as u64);
    }
}

/// Encode an `IonValue` (without BVM) into `out`.
pub fn encode_value(out: &mut Vec<u8>, val: &IonValue) {
    match val {
        IonValue::Null => out.push(0x0F),
        IonValue::Bool(b) => out.push(if *b { 0x11 } else { 0x10 }),
        IonValue::Int(v) => {
            if *v >= 0 {
                let mut tmp = Vec::with_capacity(8);
                let len = write_uint_be(&mut tmp, *v as u64);
                write_typed_header(out, 0x2, len);
                out.extend_from_slice(&tmp);
            } else {
                let mut tmp = Vec::with_capacity(8);
                let len = write_uint_be(&mut tmp, v.unsigned_abs());
                write_typed_header(out, 0x3, len);
                out.extend_from_slice(&tmp);
            }
        }
        IonValue::Decimal(d) => {
            write_typed_header(out, 0x5, d.len());
            out.extend_from_slice(d);
        }
        IonValue::Symbol(sym) => {
            let mut tmp = Vec::with_capacity(8);
            let len = write_uint_be(&mut tmp, *sym);
            write_typed_header(out, 0x7, len);
            out.extend_from_slice(&tmp);
        }
        IonValue::String(s) => {
            let bytes = s.as_bytes();
            write_typed_header(out, 0x8, bytes.len());
            out.extend_from_slice(bytes);
        }
        IonValue::Blob(b) => {
            write_typed_header(out, 0xA, b.len());
            out.extend_from_slice(b);
        }
        IonValue::List(items) => {
            let mut inner = Vec::new();
            for item in items {
                encode_value(&mut inner, item);
            }
            write_typed_header(out, 0xB, inner.len());
            out.extend_from_slice(&inner);
        }
        IonValue::Struct(fields) => {
            let mut inner = Vec::new();
            for (k, v) in fields {
                write_var_uint(&mut inner, *k);
                encode_value(&mut inner, v);
            }
            write_typed_header(out, 0xD, inner.len());
            out.extend_from_slice(&inner);
        }
        IonValue::Annotated(annots, inner_val) => {
            let mut annot_bytes = Vec::new();
            for a in annots {
                write_var_uint(&mut annot_bytes, *a);
            }
            let mut body = Vec::new();
            write_var_uint(&mut body, annot_bytes.len() as u64);
            body.extend_from_slice(&annot_bytes);
            encode_value(&mut body, inner_val);
            write_typed_header(out, 0xE, body.len());
            out.extend_from_slice(&body);
        }
    }
}

/// Encode a stream of top-level `IonValue`s prefixed with the Ion 1.0 BVM.
pub fn encode_stream(values: &[IonValue]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&ION_BVM);
    for v in values {
        encode_value(&mut out, v);
    }
    out
}

/// Decode a complete Ion 1.0 binary stream (must start with `ION_BVM`).
pub fn decode_stream(data: &[u8]) -> Result<Vec<IonValue>, String> {
    if data.len() < 4 || data[..4] != ION_BVM {
        return Err("Missing Ion 1.0 BVM".to_string());
    }
    let mut pos = 4;
    let mut out = Vec::new();
    while pos < data.len() {
        let (val, consumed) = decode_value(&data[pos..], 0)?;
        pos += consumed;
        if !matches!(val, IonValue::Null) || consumed == 1 {
            out.push(val);
        }
    }
    Ok(out)
}

fn read_uint_be(data: &[u8]) -> u64 {
    let mut v = 0u64;
    for &b in data {
        v = (v << 8) | u64::from(b);
    }
    v
}

/// Decode a single Ion 1.0 binary value, returning `(value, bytes_consumed)`.
pub fn decode_value(data: &[u8], depth: usize) -> Result<(IonValue, usize), String> {
    if depth > 64 {
        return Err("Ion nesting depth exceeded".to_string());
    }
    if data.is_empty() {
        return Err("Unexpected EOF in Ion value".to_string());
    }
    let header = data[0];
    let type_code = header >> 4;
    let nibble = header & 0x0F;
    let mut pos = 1usize;

    // Null of any type
    if nibble == 0x0F {
        return Ok((IonValue::Null, 1));
    }

    // Bool
    if type_code == 0x1 {
        return Ok((IonValue::Bool(nibble != 0), 1));
    }

    let len = if nibble == 0x0E {
        let (l, c) = read_var_uint(&data[pos..])?;
        pos += c;
        usize::try_from(l).map_err(|_| "Ion length overflow".to_string())?
    } else {
        usize::from(nibble)
    };

    if pos.saturating_add(len) > data.len() {
        return Err("Truncated Ion value payload".to_string());
    }
    let payload = &data[pos..pos + len];
    let total = pos + len;

    let val = match type_code {
        0x0 => IonValue::Null, // NOP pad
        0x2 => IonValue::Int(read_uint_be(payload) as i64),
        0x3 => IonValue::Int(-(read_uint_be(payload) as i64)),
        0x5 => IonValue::Decimal(payload.to_vec()),
        0x7 => IonValue::Symbol(read_uint_be(payload)),
        0x8 => IonValue::String(String::from_utf8_lossy(payload).into_owned()),
        0xA => IonValue::Blob(payload.to_vec()),
        0xB | 0xC => {
            let mut items = Vec::new();
            let mut sub = 0;
            while sub < payload.len() {
                let (item, c) = decode_value(&payload[sub..], depth + 1)?;
                sub += c;
                items.push(item);
            }
            IonValue::List(items)
        }
        0xD => {
            let mut fields = Vec::new();
            let mut sub = 0;
            while sub < payload.len() {
                let (k, kc) = read_var_uint(&payload[sub..])?;
                sub += kc;
                let (v, vc) = decode_value(&payload[sub..], depth + 1)?;
                sub += vc;
                fields.push((k, v));
            }
            IonValue::Struct(fields)
        }
        0xE => {
            let (annot_len, alc) = read_var_uint(payload)?;
            let annot_len =
                usize::try_from(annot_len).map_err(|_| "Ion annotation length overflow")?;
            let mut sub = alc;
            if sub + annot_len > payload.len() {
                return Err("Truncated Ion annotation".to_string());
            }
            let annot_end = sub + annot_len;
            let mut annots = Vec::new();
            while sub < annot_end {
                let (a, ac) = read_var_uint(&payload[sub..annot_end])?;
                sub += ac;
                annots.push(a);
            }
            let (inner, _) = decode_value(&payload[annot_end..], depth + 1)?;
            IonValue::Annotated(annots, Box::new(inner))
        }
        _ => IonValue::Blob(payload.to_vec()),
    };

    Ok((val, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ion_roundtrip_struct_and_annotated() {
        let val = IonValue::Annotated(
            vec![3],
            Box::new(IonValue::Struct(vec![
                (4, IonValue::String("Hello KFX".to_string())),
                (
                    7,
                    IonValue::List(vec![IonValue::Int(42), IonValue::Symbol(10)]),
                ),
            ])),
        );
        let bytes = encode_stream(&[val.clone()]);
        assert_eq!(&bytes[..4], &ION_BVM);
        let decoded = decode_stream(&bytes).unwrap();
        assert_eq!(decoded, vec![val]);
    }
}
