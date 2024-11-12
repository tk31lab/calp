use std::{
    collections::HashMap,
    env,
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Cursor, Read},
};

use ansi_term::{Colour, Style};
use chrono::{Datelike, Local, NaiveDate};
use clap::{builder::PossibleValue, Args, Parser, ValueEnum};
use consts::{
    ENGLISH_MONTH_NAMES, ENGLISH_WEEK_NAMES, JAPANESE_LUNAR_MONTH_NAMES, JAPANESE_WEEK_NAMES,
};
use encoding_rs::SHIFT_JIS;
use itertools::izip;
use months_parser::{parse_months, Months};

mod consts;
mod months_parser;

type LibResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// Selected Months(1-12) e.g. 1,3,5 1,3-5,12
    #[arg(short, value_name = "MONTHS", value_parser=parse_months)]
    months: Option<Months>,

    /// Year (1-9999)
    #[arg(value_name = "YEAR", value_parser=clap::value_parser!(i32).range(1..=9999))]
    year: Option<i32>,

    /// Show whole current year
    #[arg(short='y', long="year", conflicts_with_all=&["months", "year"])]
    cur_year: bool,

    /// Language
    #[arg(short, long, value_parser=clap::value_parser!(Lang), default_value="ja")]
    lang: Lang,

    #[command(flatten)]
    file_config: FileConfig,
}

#[derive(Debug, Args)]
struct FileConfig {
    /// Input Japanese national holiday file
    #[arg(short, long, value_name = "FILE")]
    file: Option<String>,

    /// Japanese national holiday file encoding
    #[arg(short, long, value_parser=clap::value_parser!(Encoding), default_value="sjis")]
    encoding: Encoding,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Encoding {
    ShiftJis,
    Utf8,
}

impl ValueEnum for Encoding {
    fn value_variants<'a>() -> &'a [Self] {
        &[Encoding::ShiftJis, Encoding::Utf8]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Encoding::ShiftJis => PossibleValue::new("sjis"),
            Encoding::Utf8 => PossibleValue::new("utf8"),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Lang {
    Japanese,
    English,
}

impl ValueEnum for Lang {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Japanese, Self::English]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            Lang::Japanese => PossibleValue::new("ja"),
            Lang::English => PossibleValue::new("en"),
        })
    }
}

struct FormatConfig {
    show_year: bool,
    lang: Lang,
}

struct HolidayInfo {
    info: HashMap<i32, HashMap<u32, u32>>,
}

impl HolidayInfo {
    fn new() -> HolidayInfo {
        HolidayInfo {
            info: HashMap::new(),
        }
    }

    fn is_holiday(&self, year: i32, month: u32, day: u32) -> bool {
        let b = self
            .info
            .get(&year)
            .and_then(|m| m.get(&month))
            .unwrap_or(&0);
        b & (1 << (day - 1)) != 0
    }

    fn add(&mut self, date: NaiveDate) {
        let (year, month, day) = (date.year(), date.month(), date.day());
        let m = self.info.entry(year).or_default();
        let d = m.entry(month).or_insert(0);
        *d |= 1 << (day - 1);
    }
}

pub fn run(config: Config) -> LibResult<()> {
    // println!("{:#?}", config);
    let today = Local::now().date_naive();
    let holiday_info = load_holiday_file(&config.file_config)?;
    let show_whole_year = config.cur_year || (config.year.is_some() && config.months.is_none());

    let year = config.year.unwrap_or_else(|| today.year());
    let months = if show_whole_year {
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    } else {
        config.months.unwrap_or_else(|| vec![today.month()])
    };
    let format_config = FormatConfig {
        show_year: months.len() == 1,
        lang: config.lang,
    };
    print_months(year, &months, format_config, today, &holiday_info);

    Ok(())
}

fn load_holiday_file(file_config: &FileConfig) -> LibResult<HolidayInfo> {
    let (load_default, file) = match &file_config.file {
        Some(v) => (false, v.clone()),
        None => match env::var("HOME") {
            Ok(home) => (true, format!("{home}/.calp_shuku")),
            _ => return Ok(HolidayInfo::new()),
        },
    };
    let f = match File::open(file) {
        Ok(f) => f,
        Err(e) => {
            if load_default {
                return Ok(HolidayInfo::new());
            } else {
                return Err(e.into());
            }
        }
    };

    let mut file = BufReader::new(f);
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    let s = match file_config.encoding {
        Encoding::ShiftJis => {
            let (s, _, _) = SHIFT_JIS.decode(&buf);
            s
        }
        Encoding::Utf8 => String::from_utf8_lossy(&buf), // UTF-8 is the default encoding in Rust.
    };

    let cursor = Cursor::new(s.as_bytes());
    let r = BufReader::new(cursor);
    let holidays = r
        .lines()
        .filter_map(|line| match line {
            Ok(line) => {
                let ls = line.split(",").next()?;
                NaiveDate::parse_from_str(ls, "%Y/%m/%d").ok()
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    let mut ret = HolidayInfo::new();
    for date in holidays {
        ret.add(date);
    }

    Ok(ret)
}

fn print_months(
    year: i32,
    months: &Months,
    format_config: FormatConfig,
    today: NaiveDate,
    holiday_info: &HolidayInfo,
) {
    if !format_config.show_year {
        if months.len() == 2 {
            println!("{:^40}", year);
        } else {
            println!("{:^60}", year);
        }
    }

    let v = months
        .iter()
        .map(|month| format_month(year, *month, &format_config, today, holiday_info))
        .collect::<Vec<Vec<_>>>();
    for (i, chunk) in v.chunks(3).enumerate() {
        if i > 0 {
            println!();
        }
        match chunk {
            [m1, m2, m3] => {
                for (e1, e2, e3) in izip!(m1, m2, m3) {
                    println!("{}{}{}", e1, e2, e3);
                }
            }
            [m1, m2] => {
                for (e1, e2) in izip!(m1, m2) {
                    println!("{}{}", e1, e2);
                }
            }
            [m1] => {
                println!("{}", m1.join("\n"));
            }
            _ => (),
        }
    }
}

fn last_day_in_month(year: i32, month: u32) -> NaiveDate {
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(y, m, 1)
        .and_then(|d| d.pred_opt())
        .unwrap()
}

fn format_month(
    year: i32,
    month: u32,
    format_config: &FormatConfig,
    today: NaiveDate,
    holiday_info: &HolidayInfo,
) -> Vec<String> {
    let formatted_days = format_days(year, month, today, holiday_info);

    let header = match format_config.lang {
        Lang::Japanese => format_header_jp(year, month, format_config.show_year),
        Lang::English => format_header_en(year, month, format_config.show_year),
    };

    let week_names = format!(
        "{}  ",
        match format_config.lang {
            Lang::Japanese => JAPANESE_WEEK_NAMES.join(" "),
            Lang::English => ENGLISH_WEEK_NAMES.join(" "),
        }
    );

    let mut ret = vec![header, week_names];
    ret.extend(formatted_days);

    ret
}

fn format_header_jp(year: i32, month: u32, show_year: bool) -> String {
    format!(
        "{:^17}  ",
        format!(
            "{month}月({}){}",
            JAPANESE_LUNAR_MONTH_NAMES[month as usize - 1],
            if show_year {
                format!(" {year}")
            } else {
                "".to_string()
            }
        )
    )
}

fn format_header_en(year: i32, month: u32, show_year: bool) -> String {
    format!(
        "{:^20}  ",
        format!(
            "{}{}",
            ENGLISH_MONTH_NAMES[month as usize - 1],
            if show_year {
                format!(" {year}")
            } else {
                "".to_string()
            }
        )
    )
}

fn format_days(year: i32, month: u32, today: NaiveDate, holiday_info: &HolidayInfo) -> Vec<String> {
    let is_today = |d: u32| year == today.year() && month == today.month() && d == today.day();
    let days = preformat_days(year, month);
    days.chunks(7)
        .map(|d| {
            let s = d
                .iter()
                .enumerate()
                .map(|(i, d)| {
                    if *d == 0 {
                        "  ".to_string()
                    } else {
                        let s = format!("{:>2}", d);
                        Some(Style::new())
                            .map(|v| {
                                if i == 0 || holiday_info.is_holiday(year, month, *d) {
                                    v.fg(Colour::Red)
                                } else if i == 6 {
                                    v.fg(Colour::Blue)
                                } else {
                                    v
                                }
                            })
                            .map(|v| if is_today(*d) { v.reverse() } else { v })
                            .map(|v| v.paint(&s).to_string())
                            .unwrap_or(s)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("{}  ", s)
        })
        .collect::<Vec<_>>()
}

fn preformat_days(year: i32, month: u32) -> Vec<u32> {
    let last = last_day_in_month(year, month);
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let mut days = vec![0; 7 * 6];
    let first_weekday = first.weekday().num_days_from_sunday() as usize;
    days.splice(
        first_weekday..first_weekday + last.day() as usize,
        (1..=last.day()).collect::<Vec<_>>(),
    );
    days
}

#[cfg(test)]
mod test {
    use crate::preformat_days;

    #[test]
    fn test_preformat_days() {
        // start Su
        let res = preformat_days(2024, 12);
        let mut cmp = vec![];
        cmp.extend((1..=31).collect::<Vec<_>>());
        cmp.extend(vec![0; 4 + 7]);
        assert_eq!(res, cmp);

        // start Mo
        let res = preformat_days(2024, 7);
        let mut cmp = vec![0; 1];
        cmp.extend((1..=31).collect::<Vec<_>>());
        cmp.extend(vec![0; 3 + 7]);
        assert_eq!(res, cmp);

        // start Tu
        let res = preformat_days(2024, 10);
        let mut cmp = vec![0; 2];
        cmp.extend((1..=31).collect::<Vec<_>>());
        cmp.extend(vec![0; 2 + 7]);
        assert_eq!(res, cmp);

        // start We
        let res = preformat_days(2024, 5);
        let mut cmp = vec![0; 3];
        cmp.extend((1..=31).collect::<Vec<_>>());
        cmp.extend(vec![0; 1 + 7]);
        assert_eq!(res, cmp);

        // start Th
        let res = preformat_days(2024, 8);
        let mut cmp = vec![0; 4];
        cmp.extend((1..=31).collect::<Vec<_>>());
        cmp.extend(vec![0; 7]);
        assert_eq!(res, cmp);

        // start Fr
        let res = preformat_days(2024, 3);
        let mut cmp = vec![0; 5];
        cmp.extend((1..=31).collect::<Vec<_>>());
        cmp.extend(vec![0; 6]);
        assert_eq!(res, cmp);

        // start Sa
        let res = preformat_days(2024, 6);
        let mut cmp = vec![0; 6];
        cmp.extend((1..=30).collect::<Vec<_>>());
        cmp.extend(vec![0; 6]);
        assert_eq!(res, cmp);
    }
}
