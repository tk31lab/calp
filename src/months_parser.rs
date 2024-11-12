use std::{
    cmp::{max, min},
    error::Error,
    ops::Range,
};

use regex::Regex;

pub type Months = Vec<u32>;

pub fn parse_months(value: &str) -> Result<Months, String> {
    let range = value
        .split(",")
        .map(|ele| {
            let (s, e) = parse_range(ele)
                .map(|r| (r.start, r.end))
                .map_err(|_| format!("illegal list value: \"{ele}\""))?;

            let s = match s {
                Some(v) if (1..=12).contains(&v) => v,
                _ => {
                    // 範囲外
                    return Err(format!(
                        "invalid month: \"{}\"",
                        s.map_or_else(|| ele.to_string(), |v| v.to_string())
                    ));
                }
            };

            let e = match e {
                Some(e) if !(1..=12).contains(&e) => {
                    return Err(format!("invalid month: \"{}\"", e));
                }
                Some(e) if s >= e => {
                    return Err(format!(
                        "First month in range ({s}) must be lower than second month ({e})"
                    ));
                }
                Some(e) => e,
                None => s,
            };

            Ok((s - 1)..e)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let range = sort_month_range_list(&range);
    let m12: [u32; 12] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let mut offset = 0;
    let months = range
        .iter()
        .map(|range| {
            let (next, a) = extract_month(&m12, offset, range.clone());
            offset = next;
            a.iter().copied()
        })
        .flatten()
        .collect::<Vec<_>>();

    Ok(months)
}

fn parse_range(range: &str) -> Result<Range<Option<usize>>, Box<dyn Error>> {
    let re = Regex::new(r#"^(\d+)(-(\d+)){0,1}$"#)?;
    let caps = re.captures(range);
    match &caps {
        Some(caps) => {
            let s = Some((&caps[1]).parse()?);
            let e = caps
                .get(3)
                .map_or_else::<Result<Option<usize>, Box<dyn Error>>, _, _>(
                    || Ok(None),
                    |e| Ok(Some(e.as_str().parse()?)),
                )?;
            Ok(s..e)
        }
        None => Err("illegal range format".into()),
    }
}

fn sort_month_range_list(l: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut l = l.iter().map(|e| e.clone()).collect::<Vec<_>>();
    l.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.end.cmp(&b.end)));
    l
}

fn extract_month(arr: &[u32], offset: usize, range: Range<usize>) -> (usize, &[u32]) {
    let s = max(offset, range.start);
    let e = min(arr.len(), range.end);
    if s > e {
        (e, &[])
    } else {
        (e, &arr[s..e])
    }
}
