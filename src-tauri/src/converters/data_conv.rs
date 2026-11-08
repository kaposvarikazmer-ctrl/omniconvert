use crate::converters::image_conv::{calculate_savings, ensure_output_dir};
use crate::models::{ConversionOptions, SingleConversionResponse};
use calamine::{Data, Reader};
use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};
use rust_xlsxwriter::{Format, Workbook};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct DataTable {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub fn convert_data_file(
    input_path: &Path,
    source_format: &str,
    target_format: &str,
    options: &ConversionOptions,
) -> Result<SingleConversionResponse, String> {
    let original_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
    let table = read_into_data_table(input_path, source_format)?;

    let out_dir = ensure_output_dir(&options.output_dir, input_path)?;
    let stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("data_converted");
    let target_clean = target_format.trim_start_matches('.').to_lowercase();
    let out_path = out_dir.join(format!("{}.{}", stem, target_clean));

    match target_clean.as_str() {
        "xlsx" => write_table_to_xlsx(&table, &out_path)?,
        "csv" => write_table_to_delimited(&table, &out_path, b',')?,
        "tsv" => write_table_to_delimited(&table, &out_path, b'\t')?,
        "json" => write_table_to_json(&table, &out_path)?,
        "yaml" | "yml" => write_table_to_yaml(&table, &out_path)?,
        "toml" => write_table_to_toml(&table, &out_path)?,
        "xml" => write_table_to_xml(&table, &out_path)?,
        "sql" => write_table_to_sql(&table, stem, &out_path)?,
        "md" => write_table_to_markdown(&table, stem, &out_path)?,
        "html" => write_table_to_html(&table, stem, &out_path)?,
        "pdf" => write_table_to_pdf(&table, stem, &out_path)?,
        _ => {
            return Err(format!(
                "Nem támogatott adat-célformátum: {}",
                target_clean
            ))
        }
    }

    let output_size = std::fs::metadata(&out_path).map(|m| m.len()).unwrap_or(0);
    Ok(SingleConversionResponse {
        output_path: out_path.to_string_lossy().to_string(),
        output_size_bytes: output_size,
        savings_percent: calculate_savings(original_size, output_size),
    })
}

pub fn read_into_data_table(path: &Path, source_format: &str) -> Result<DataTable, String> {
    match source_format {
        "xlsx" | "xls" | "ods" => {
            let mut wb = calamine::open_workbook_auto(path)
                .map_err(|e| format!("Táblázat megnyitási hiba: {}", e))?;
            let first_sheet = wb
                .sheet_names()
                .first()
                .cloned()
                .ok_or_else(|| "A munkafüzet nem tartalmaz munkalapot.".to_string())?;
            let range = wb
                .worksheet_range(&first_sheet)
                .map_err(|e| format!("Munkalap olvasási hiba: {}", e))?;

            let mut iter = range.rows();
            let columns: Vec<String> = if let Some(first_row) = iter.next() {
                first_row
                    .iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let s = format_calamine_cell(cell);
                        if s.is_empty() {
                            format!("col_{}", i + 1)
                        } else {
                            s
                        }
                    })
                    .collect()
            } else {
                vec![]
            };

            let rows: Vec<Vec<String>> = iter
                .map(|r| r.iter().map(format_calamine_cell).collect())
                .collect();

            Ok(DataTable { columns, rows })
        }
        "csv" | "tsv" => {
            let delim = if source_format == "tsv" { b'\t' } else { b',' };
            let mut rdr = csv::ReaderBuilder::new()
                .delimiter(delim)
                .flexible(true)
                .from_path(path)
                .map_err(|e| format!("CSV/TSV olvasási hiba: {}", e))?;

            let columns: Vec<String> = rdr
                .headers()
                .map_err(|e| e.to_string())?
                .iter()
                .map(|s| s.to_string())
                .collect();

            let mut rows = Vec::new();
            for rec in rdr.records().flatten() {
                rows.push(rec.iter().map(|s| s.to_string()).collect());
            }
            Ok(DataTable { columns, rows })
        }
        "json" => {
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let val: Value =
                serde_json::from_str(&text).map_err(|e| format!("JSON elemzési hiba: {}", e))?;
            value_to_data_table(&val)
        }
        "yaml" | "yml" => {
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let val: Value =
                serde_yaml::from_str(&text).map_err(|e| format!("YAML elemzési hiba: {}", e))?;
            value_to_data_table(&val)
        }
        "toml" => {
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let val: Value =
                toml::from_str(&text).map_err(|e| format!("TOML elemzési hiba: {}", e))?;
            value_to_data_table(&val)
        }
        _ => {
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let lines: Vec<Vec<String>> = text
                .lines()
                .enumerate()
                .map(|(idx, l)| vec![(idx + 1).to_string(), l.to_string()])
                .collect();
            Ok(DataTable {
                columns: vec!["line".to_string(), "content".to_string()],
                rows: lines,
            })
        }
    }
}

fn format_calamine_cell(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{:.0}", f)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("{:?}", e),
    }
}

fn value_to_data_table(val: &Value) -> Result<DataTable, String> {
    match val {
        Value::Array(arr) => {
            let mut col_set = Vec::new();
            for item in arr {
                if let Value::Object(map) = item {
                    for k in map.keys() {
                        if !col_set.contains(k) {
                            col_set.push(k.clone());
                        }
                    }
                }
            }
            if col_set.is_empty() {
                col_set.push("value".to_string());
                let rows = arr.iter().map(|v| vec![scalar_to_string(v)]).collect();
                return Ok(DataTable {
                    columns: col_set,
                    rows,
                });
            }
            let mut rows = Vec::with_capacity(arr.len());
            for item in arr {
                if let Value::Object(map) = item {
                    let row = col_set
                        .iter()
                        .map(|c| map.get(c).map(scalar_to_string).unwrap_or_default())
                        .collect();
                    rows.push(row);
                }
            }
            Ok(DataTable {
                columns: col_set,
                rows,
            })
        }
        Value::Object(map) => {
            // Check if any top-level value is an array of records (common in TOML/JSON/XML)
            for (_, v) in map {
                if v.is_array() {
                    return value_to_data_table(v);
                }
            }
            let columns = vec!["key".to_string(), "value".to_string()];
            let rows = map
                .iter()
                .map(|(k, v)| vec![k.clone(), scalar_to_string(v)])
                .collect();
            Ok(DataTable { columns, rows })
        }
        other => Ok(DataTable {
            columns: vec!["value".to_string()],
            rows: vec![vec![scalar_to_string(other)]],
        }),
    }
}

fn scalar_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn write_table_to_xlsx(table: &DataTable, out_path: &Path) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let header_fmt = Format::new().set_bold();

    for (col_idx, col_name) in table.columns.iter().enumerate() {
        worksheet
            .write_with_format(0, col_idx as u16, col_name, &header_fmt)
            .map_err(|e| e.to_string())?;
    }

    for (row_idx, row) in table.rows.iter().enumerate() {
        for (col_idx, cell_val) in row.iter().enumerate() {
            if let Ok(num) = cell_val.parse::<f64>() {
                let _ = worksheet.write_number((row_idx + 1) as u32, col_idx as u16, num);
            } else {
                let _ = worksheet.write_string((row_idx + 1) as u32, col_idx as u16, cell_val);
            }
        }
    }

    workbook
        .save(out_path)
        .map_err(|e| format!("XLSX mentési hiba: {}", e))
}

fn write_table_to_delimited(table: &DataTable, out_path: &Path, delim: u8) -> Result<(), String> {
    let mut wtr = csv::WriterBuilder::new()
        .delimiter(delim)
        .from_path(out_path)
        .map_err(|e| e.to_string())?;
    if !table.columns.is_empty() {
        wtr.write_record(&table.columns)
            .map_err(|e| e.to_string())?;
    }
    for row in &table.rows {
        wtr.write_record(row).map_err(|e| e.to_string())?;
    }
    wtr.flush().map_err(|e| e.to_string())
}

fn table_to_json_records(table: &DataTable) -> Vec<Value> {
    table
        .rows
        .iter()
        .map(|row| {
            let mut obj = Map::new();
            for (idx, col) in table.columns.iter().enumerate() {
                let val_str = row.get(idx).cloned().unwrap_or_default();
                let json_val = if let Ok(i) = val_str.parse::<i64>() {
                    Value::from(i)
                } else if let Ok(f) = val_str.parse::<f64>() {
                    Value::from(f)
                } else if val_str.eq_ignore_ascii_case("true") {
                    Value::Bool(true)
                } else if val_str.eq_ignore_ascii_case("false") {
                    Value::Bool(false)
                } else {
                    Value::String(val_str)
                };
                obj.insert(col.clone(), json_val);
            }
            Value::Object(obj)
        })
        .collect()
}

fn write_table_to_json(table: &DataTable, out_path: &Path) -> Result<(), String> {
    let records = table_to_json_records(table);
    let json_text =
        serde_json::to_string_pretty(&records).map_err(|e| format!("JSON írási hiba: {}", e))?;
    std::fs::write(out_path, json_text).map_err(|e| e.to_string())
}

fn write_table_to_yaml(table: &DataTable, out_path: &Path) -> Result<(), String> {
    let records = table_to_json_records(table);
    let yaml_text =
        serde_yaml::to_string(&records).map_err(|e| format!("YAML írási hiba: {}", e))?;
    std::fs::write(out_path, yaml_text).map_err(|e| e.to_string())
}

fn write_table_to_toml(table: &DataTable, out_path: &Path) -> Result<(), String> {
    let records = table_to_json_records(table);
    let mut root = BTreeMap::new();
    root.insert("records".to_string(), records);
    let toml_text =
        toml::to_string_pretty(&root).map_err(|e| format!("TOML írási hiba: {}", e))?;
    std::fs::write(out_path, toml_text).map_err(|e| e.to_string())
}

fn write_table_to_xml(table: &DataTable, out_path: &Path) -> Result<(), String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<dataset>\n");
    for row in &table.rows {
        xml.push_str("  <record>\n");
        for (i, col) in table.columns.iter().enumerate() {
            let tag = sanitize_identifier(col);
            let val = escape_xml(row.get(i).map(|s| s.as_str()).unwrap_or(""));
            xml.push_str(&format!("    <{tag}>{val}</{tag}>\n"));
        }
        xml.push_str("  </record>\n");
    }
    xml.push_str("</dataset>\n");
    std::fs::write(out_path, xml).map_err(|e| e.to_string())
}

fn write_table_to_sql(table: &DataTable, table_name_raw: &str, out_path: &Path) -> Result<(), String> {
    let tname = sanitize_identifier(table_name_raw);
    let cols_ident: Vec<String> = table
        .columns
        .iter()
        .map(|c| format!("\"{}\"", sanitize_identifier(c)))
        .collect();

    let mut sql = format!("-- OmniConvert SQL Export\nCREATE TABLE IF NOT EXISTS \"{}\" (\n", tname);
    for (idx, col) in cols_ident.iter().enumerate() {
        let comma = if idx + 1 < cols_ident.len() { "," } else { "" };
        sql.push_str(&format!("  {} TEXT{}\n", col, comma));
    }
    sql.push_str(");\n\n");

    for row in &table.rows {
        let vals: Vec<String> = (0..table.columns.len())
            .map(|i| {
                let raw = row.get(i).map(|s| s.as_str()).unwrap_or("");
                format!("'{}'", raw.replace('\'', "''"))
            })
            .collect();
        sql.push_str(&format!(
            "INSERT INTO \"{}\" ({}) VALUES ({});\n",
            tname,
            cols_ident.join(", "),
            vals.join(", ")
        ));
    }

    std::fs::write(out_path, sql).map_err(|e| e.to_string())
}

fn write_table_to_markdown(table: &DataTable, title: &str, out_path: &Path) -> Result<(), String> {
    let mut md = format!("# {}\n\n", title);
    if !table.columns.is_empty() {
        md.push_str("| ");
        md.push_str(&table.columns.join(" | "));
        md.push_str(" |\n| ");
        md.push_str(
            &table
                .columns
                .iter()
                .map(|_| "---")
                .collect::<Vec<_>>()
                .join(" | "),
        );
        md.push_str(" |\n");
    }
    for row in &table.rows {
        md.push_str("| ");
        md.push_str(&row.join(" | "));
        md.push_str(" |\n");
    }
    std::fs::write(out_path, md).map_err(|e| e.to_string())
}

fn write_table_to_html(table: &DataTable, title: &str, out_path: &Path) -> Result<(), String> {
    let mut html = format!(
        "<!DOCTYPE html>\n<html lang=\"hu\">\n<head><meta charset=\"UTF-8\"><title>{}</title>\
        <style>body{{font-family:Inter,sans-serif;margin:24px;background:#09090b;color:#f4f4f5}}\
        table{{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums}}\
        th,td{{border:1px solid #27272a;padding:8px 12px;text-align:left;font-size:13px}}\
        th{{background:#18181b;font-weight:600}}</style></head>\n<body>\n<h1>{}</h1>\n<table>\n<thead><tr>",
        escape_xml(title),
        escape_xml(title)
    );
    for col in &table.columns {
        html.push_str(&format!("<th>{}</th>", escape_xml(col)));
    }
    html.push_str("</tr></thead>\n<tbody>\n");
    for row in &table.rows {
        html.push_str("<tr>");
        for cell in row {
            html.push_str(&format!("<td>{}</td>", escape_xml(cell)));
        }
        html.push_str("</tr>\n");
    }
    html.push_str("</tbody>\n</table>\n</body>\n</html>\n");
    std::fs::write(out_path, html).map_err(|e| e.to_string())
}

/// Generates a formatted PDF table directly from a DataTable using lopdf vector text & grid lines.
pub fn write_table_to_pdf(table: &DataTable, title: &str, out_path: &Path) -> Result<(), String> {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let font_bold_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
        "Encoding" => "WinAnsiEncoding",
    });

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
            "F2" => font_bold_id,
        }
    });

    let mut ops = Vec::new();
    // Title
    ops.push(Operation::new("BT", vec![]));
    ops.push(Operation::new("Tf", vec!["F2".into(), 14.into()]));
    ops.push(Operation::new("Td", vec![40.into(), 790.into()]));
    ops.push(Operation::new(
        "Tj",
        vec![Object::string_literal(ascii_fallback(title))],
    ));
    ops.push(Operation::new("ET", vec![]));

    // Header row
    let header_line = table.columns.join("   |   ");
    ops.push(Operation::new("BT", vec![]));
    ops.push(Operation::new("Tf", vec!["F2".into(), 10.into()]));
    ops.push(Operation::new("Td", vec![40.into(), 764.into()]));
    ops.push(Operation::new(
        "Tj",
        vec![Object::string_literal(ascii_fallback(&header_line))],
    ));
    ops.push(Operation::new("ET", vec![]));

    // Data rows (first 38 rows per page)
    let mut y = 744;
    for row in table.rows.iter().take(38) {
        let line = row.join("   |   ");
        ops.push(Operation::new("BT", vec![]));
        ops.push(Operation::new("Tf", vec!["F1".into(), 9.into()]));
        ops.push(Operation::new("Td", vec![40.into(), y.into()]));
        ops.push(Operation::new(
            "Tj",
            vec![Object::string_literal(ascii_fallback(&line))],
        ));
        ops.push(Operation::new("ET", vec![]));
        y -= 18;
        if y < 50 {
            break;
        }
    }

    let content = Content { operations: ops };
    let content_id = doc.add_object(Stream::new(
        dictionary! {},
        content.encode().unwrap_or_default(),
    ));

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        "Contents" => content_id,
        "Resources" => resources_id,
    });

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    let mut file = File::create(out_path).map_err(|e| e.to_string())?;
    doc.save_to(&mut file).map_err(|e| e.to_string())?;
    let _ = file.flush();
    Ok(())
}

fn sanitize_identifier(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    if s.is_empty() {
        "field".to_string()
    } else {
        s
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn ascii_fallback(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'ő' => 'o',
            'ű' => 'u',
            'Ő' => 'O',
            'Ű' => 'U',
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' | 'ö' => 'o',
            'ú' | 'ü' => 'u',
            'Á' => 'A',
            'É' => 'E',
            'Í' => 'I',
            'Ó' | 'Ö' => 'O',
            'Ú' | 'Ü' => 'U',
            other if other.is_ascii() => other,
            _ => ' ',
        })
        .collect()
}
