use aviutl2::anyhow::{Context, Result, anyhow};
use regex::Regex;

const EXCLUDE_EFFECT: &[&str] = &[
    "オーディオバッファ",
    "カメラ制御",
    "グループ制御(音声)",
    "フィルタ効果",
    "音声ファイル",
    "時間制御(オブジェクト)",
];

pub fn get_last_object_index(text: &str) -> Option<usize> {
    let re = Regex::new(r"^\[Object\.(\d+)\]").unwrap();

    text.lines()
        .filter_map(|line| {
            re.captures(line)
                .and_then(|cap| cap.get(1))
                .and_then(|m| m.as_str().parse::<usize>().ok())
        })
        .max()
}

pub fn get_effect_name(input: &str) -> Option<String> {
    let mut in_target_section = false;

    for line in input.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') {
            in_target_section = line == "[Object.0]";
            continue;
        }
        if in_target_section && line.starts_with("effect.name=") {
            return Some(line["effect.name=".len()..].to_string());
        }
    }
    None
}

pub fn ensure_effect(
    edit_section: &mut aviutl2::generic::EditSection,
    effect_name: &str,
    effect_alias: &str,
) -> Result<()> {
    let obj = edit_section
        .get_focused_object()
        .context("get_focused_object failed")?;
    let obj = match obj {
        Some(o) => o,
        None => {
            return Ok(());
        }
    };
    let object = edit_section.object(obj);
    let mut alias = object.get_alias().context("get_alias failed")?;
    let effect_count = object
        .count_effect(effect_name)
        .context("count_effect failed")?;

    if effect_count == 0 {
        let effect_name =
            get_effect_name(&alias).ok_or_else(|| anyhow!("effect name not found"))?;

        if EXCLUDE_EFFECT.contains(&effect_name.as_str()) {
            return Ok(());
        }

        let next_index = get_last_object_index(&alias).unwrap_or(0) + 1;
        let new_section = format!(
            r#"
[Object.{}]"#,
            next_index
        );
        alias.push_str(&new_section);
        alias.push_str(effect_alias);

        let layer_frame = object.get_layer_frame().context("get_layer_frame failed")?;
        edit_section
            .delete_object(obj)
            .context("delete_object failed")?;
        let created_object = edit_section
            .create_object_from_alias(
                &alias,
                layer_frame.layer,
                layer_frame.start,
                layer_frame.end - layer_frame.start,
            )
            .context("create_object_from_alias failed")?;
        edit_section
            .focus_object(created_object)
            .context("focus_object failed")?;
    }

    Ok(())
}
