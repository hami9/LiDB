use crate::{error, integer, State, MAX_DEVICES};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct CpuCounters(pub(crate) [Option<u64>; 10]);

impl CpuCounters {
    pub(crate) fn accounting(&self) -> Option<[u64; 8]> {
        let mut result = [0; 8];
        for (target, source) in result.iter_mut().zip(self.0) {
            *target = source?;
        }
        Some(result)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TransferCounters(pub(crate) [u64; 2]);

#[derive(Clone, Debug)]
pub(crate) struct NetworkCounters {
    pub(crate) bytes: TransferCounters,
    pub(crate) errors: [u64; 2],
    pub(crate) drops: [u64; 2],
}

pub(crate) fn cpu(text: &str) -> Result<CpuCounters, State> {
    let mut aggregate = None;
    for line in text.lines() {
        let mut words = line.split_whitespace();
        if words.next() != Some("cpu") {
            continue;
        }
        if aggregate.is_some() {
            return Err(error("duplicate aggregate CPU line"));
        }
        let mut values = [None; 10];
        let mut count = 0;
        for word in words {
            let value = unsigned(word)?;
            if count < values.len() {
                values[count] = Some(value);
            }
            count += 1;
        }
        if count < 4 {
            return Err(error("aggregate CPU line lacks required counters"));
        }
        // Reject a total that cannot fit the exact counter domain.
        values[..8]
            .iter()
            .flatten()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value))
            .ok_or_else(|| error("aggregate CPU counters overflow"))?;
        aggregate = Some(CpuCounters(values));
    }
    aggregate.ok_or_else(|| error("aggregate CPU line is missing"))
}

pub(crate) fn memory(text: &str) -> BTreeMap<&'static str, State> {
    let mut values = BTreeMap::new();
    for line in text.lines() {
        let Some((name, tail)) = line.split_once(':') else {
            if let Some(name) = line.split_whitespace().next().and_then(memory_key) {
                values.insert(name, error("memory field is missing its colon"));
            }
            continue;
        };
        let Some(name) = memory_key(name) else {
            continue;
        };
        let result = || {
            let mut words = tail.split_whitespace();
            let quantity = unsigned(
                words
                    .next()
                    .ok_or_else(|| error("memory value is missing"))?,
            )?;
            if words.next() != Some("kB") || words.next().is_some() {
                return Err(error("memory quantity has invalid units or fields"));
            }
            quantity
                .checked_mul(1024)
                .ok_or_else(|| error("memory byte quantity overflows"))
        };
        let state = if values.contains_key(name) {
            error("duplicate memory field")
        } else {
            result().map_or_else(|state| state, integer)
        };
        values.insert(name, state);
    }
    if values.is_empty() {
        for name in ["MemTotal", "MemAvailable", "SwapTotal", "SwapFree"] {
            values.insert(name, error("memory source has no recognized fields"));
        }
    }
    values
}

fn memory_key(name: &str) -> Option<&'static str> {
    match name {
        "MemTotal" => Some("MemTotal"),
        "MemAvailable" => Some("MemAvailable"),
        "SwapTotal" => Some("SwapTotal"),
        "SwapFree" => Some("SwapFree"),
        _ => None,
    }
}

pub(crate) fn load(text: &str) -> Result<[f64; 3], State> {
    let mut words = text.split_whitespace();
    let mut result = [0.0; 3];
    for value in &mut result {
        *value = nonnegative(
            words
                .next()
                .ok_or_else(|| error("load average is incomplete"))?,
        )?;
    }
    let processes = words
        .next()
        .ok_or_else(|| error("load process counts are missing"))?;
    let (running, total) = processes
        .split_once('/')
        .ok_or_else(|| error("load process counts are malformed"))?;
    if unsigned(running)? > unsigned(total)? {
        return Err(error("running process count exceeds total"));
    }
    unsigned(
        words
            .next()
            .ok_or_else(|| error("load PID field is missing"))?,
    )?;
    if words.next().is_some() {
        return Err(error("load average has extra fields"));
    }
    Ok(result)
}

pub(crate) fn uptime(text: &str) -> Result<f64, State> {
    let mut words = text.split_whitespace();
    let uptime = nonnegative(words.next().ok_or_else(|| error("uptime is missing"))?)?;
    nonnegative(words.next().ok_or_else(|| error("idle time is missing"))?)?;
    if words.next().is_some() {
        return Err(error("uptime has extra fields"));
    }
    Ok(uptime)
}

pub(crate) fn pressure(text: &str) -> Result<f64, State> {
    let mut result = None;
    for line in text.lines() {
        let mut words = line.split_whitespace();
        if words.next() != Some("some") {
            continue;
        }
        if result.is_some() {
            return Err(error("duplicate pressure some line"));
        }
        let mut avg10 = None;
        let mut count = 0;
        let mut seen = [false; 4];
        for word in words {
            let (key, value) = word
                .split_once('=')
                .ok_or_else(|| error("pressure field is malformed"))?;
            let index = match key {
                "avg10" => 0,
                "avg60" => 1,
                "avg300" => 2,
                "total" => 3,
                _ => return Err(error("pressure field is unknown")),
            };
            if seen[index] {
                return Err(error("pressure field is duplicated"));
            }
            seen[index] = true;
            match key {
                "avg10" => {
                    avg10 = Some(percent(value)?);
                }
                "avg60" | "avg300" => {
                    percent(value)?;
                }
                "total" => {
                    unsigned(value)?;
                }
                _ => unreachable!("pressure keys validated above"),
            }
            count += 1;
        }
        if count != 4 {
            return Err(error("pressure some line lacks required fields"));
        }
        result = avg10;
    }
    result.ok_or_else(|| error("pressure some avg10 is missing"))
}

pub(crate) fn network(text: &str) -> Result<BTreeMap<String, NetworkCounters>, State> {
    let mut lines = text.lines();
    let first = lines
        .next()
        .ok_or_else(|| error("network headers are missing"))?;
    let second = lines
        .next()
        .ok_or_else(|| error("network headers are missing"))?;
    if !first.contains("Inter-")
        || !first.contains("Receive")
        || !first.contains("Transmit")
        || !second.contains("face")
        || !second.contains("bytes")
    {
        return Err(error("network headers are malformed"));
    }
    let mut devices = BTreeMap::new();
    for line in lines.filter(|line| !line.trim().is_empty()) {
        let (name, tail) = line
            .split_once(':')
            .ok_or_else(|| error("network device line is malformed"))?;
        let name = safe_name(name.trim())?;
        let words: Vec<_> = tail.split_whitespace().collect();
        if words.len() != 16 {
            return Err(error("network device lacks required counters"));
        }
        let counters: Vec<u64> = words
            .iter()
            .map(|word| unsigned(word))
            .collect::<Result<_, _>>()?;
        insert(
            &mut devices,
            &name,
            NetworkCounters {
                bytes: TransferCounters([counters[0], counters[8]]),
                errors: [counters[2], counters[10]],
                drops: [counters[3], counters[11]],
            },
        )?;
    }
    Ok(devices)
}

pub(crate) fn disks(text: &str) -> Result<BTreeMap<String, TransferCounters>, State> {
    let mut devices = BTreeMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let words: Vec<_> = line.split_whitespace().collect();
        if words.len() < 14 {
            return Err(error("disk line lacks required counters"));
        }
        unsigned(words[0])?;
        unsigned(words[1])?;
        let name = safe_name(words[2])?;
        for word in &words[3..] {
            unsigned(word)?;
        }
        let read_bytes = unsigned(words[5])?
            .checked_mul(512)
            .ok_or_else(|| error("disk read byte counter overflows"))?;
        let write_bytes = unsigned(words[9])?
            .checked_mul(512)
            .ok_or_else(|| error("disk write byte counter overflows"))?;
        insert(
            &mut devices,
            &name,
            TransferCounters([read_bytes, write_bytes]),
        )?;
    }
    Ok(devices)
}

fn insert<T>(devices: &mut BTreeMap<String, T>, name: &str, counters: T) -> Result<(), State> {
    if devices.len() >= MAX_DEVICES {
        return Err(error("source exceeds the device limit"));
    }
    if devices.insert(name.into(), counters).is_some() {
        return Err(error("source contains duplicate device names"));
    }
    Ok(())
}

fn safe_name(value: &str) -> Result<String, State> {
    if value.is_empty()
        || value.len() > 100
        || value.chars().any(|character| {
            character.is_control() || character.is_whitespace() || matches!(character, '/' | ':')
        })
    {
        return Err(error("source contains an unsafe device name"));
    }
    // Keep common Linux names readable and encode all other UTF-8 bytes.
    // Escaping '%' itself makes the encoding reversible and collision-free.
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write;
            write!(&mut encoded, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    if encoded.len() > 100 {
        return Err(error("encoded device name exceeds the length limit"));
    }
    Ok(encoded)
}

fn unsigned(value: &str) -> Result<u64, State> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(error("counter is not an unsigned integer"));
    }
    value.parse().map_err(|_| error("counter overflows"))
}

fn nonnegative(value: &str) -> Result<f64, State> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| error("quantity is not a number"))?;
    if !parsed.is_finite() || parsed < 0.0 || value.starts_with('-') {
        return Err(error("quantity is negative or non-finite"));
    }
    Ok(parsed)
}

fn percent(value: &str) -> Result<f64, State> {
    let parsed = nonnegative(value)?;
    if parsed > 100.0 {
        return Err(error("percentage exceeds 100"));
    }
    Ok(parsed)
}
