use crate::settings::{read_settings, write_settings};
use anyhow::{Context, Result};
use aviutl2::config::translate as tr;
use aviutl2_eframe::{
    AviUtl2EframeHandle, eframe,
    egui::{self},
};
use regex::Regex;

macro_rules! icon {
    ($name:expr $(, $key:ident = $value:expr )* $(,)?) => {
        egui::ImageSource::Bytes {
            uri: (concat!("iconify://", $name, ".svg")).into(),
            bytes: egui::load::Bytes::Static(iconify::svg!(
                $name
                $(, $key = $value )*
            ).as_bytes())
        }
    };
}

struct UiConfig {}

impl UiConfig {
    pub const BUTTON_SCALE_MAX: u32 = 200;
    pub const BUTTON_SCALE_MIN: u32 = 100;
    pub const GRID_SPACING: [f32; 2] = [1.0, 1.0];
    pub const HEADER_COLLAPSED_HEIGHT: f32 = 8.0;
    pub const HEADER_BUTTON_SIZE: [f32; 2] = [15.0, 15.0];
    pub const HEADER_ITEM_SPACING: f32 = 2.0;
    pub const COORD_BUTTON_PADDING: [f32; 2] = [0.0, 0.0];
    pub const COORD_BUTTON_SIZE: [f32; 2] = [25.0, 25.0];
    pub const MIN_COL_HEIGHT: f32 = 10.0;
    pub const MIN_COL_WIDTH: f32 = 10.0;
    pub const MODAL_PADDING: f32 = 20.0;
}

enum Coord {
    LeftTop,
    Top,
    RightTop,
    Left,
    Center,
    Right,
    LeftBottom,
    Bottom,
    RightBottom,
}

impl Coord {
    fn as_str(&self) -> &'static str {
        match self {
            Coord::LeftTop => "左上",
            Coord::Top => "上",
            Coord::RightTop => "右上",
            Coord::Left => "左",
            Coord::Center => "中心",
            Coord::Right => "右",
            Coord::LeftBottom => "左下",
            Coord::Bottom => "下",
            Coord::RightBottom => "右下",
        }
    }
}

struct AppConfig {
    pub coord: Coord,
    pub coord_button_size: [f32; 2],
    pub is_coord_button_clicked: bool,
    pub is_header_expanded: bool,
    pub show_settings_window: bool,
    pub settings: crate::settings::Settings,
}

impl AppConfig {
    const SETTINGS_FILE_PATH: &'static str = "./Plugin/中心ずらし_A_settings.json";
    const EFFECT_NAME: &'static str = "中心ずらし_A";
    const EFFECT_ALIAS: &'static str = r#"
effect.name=中心ずらし_A
位置=中心
オフセットX=0.00
オフセットY=0.00
オブジェクトを移動=0
その他.hide=1
PI="#;

    fn new() -> Self {
        Self {
            coord: Coord::Center,
            coord_button_size: UiConfig::COORD_BUTTON_SIZE,
            is_coord_button_clicked: false,
            is_header_expanded: false,
            show_settings_window: false,
            settings: crate::settings::Settings::new(),
        }
    }
}

const EXCLUDE_EFFECT: &[&str] = &[
    "オーディオバッファ",
    "カメラ制御",
    "グループ制御(音声)",
    "フィルタ効果",
    "音声ファイル",
    "時間制御(オブジェクト)",
];

fn get_last_object_index(text: &str) -> Option<usize> {
    let re = Regex::new(r"^\[Object\.(\d+)\]").unwrap();

    text.lines()
        .filter_map(|line| {
            re.captures(line)
                .and_then(|cap| cap.get(1))
                .and_then(|m| m.as_str().parse::<usize>().ok())
        })
        .max()
}

fn get_effect_name(input: &str) -> Option<String> {
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

fn ensure_effect(
    edit_section: &mut aviutl2::generic::EditSection,
    effect_name: &str,
    effect_alias: &str,
) -> Result<()> {
    let obj = edit_section
        .get_focused_object()
        .context("get_focused_object failed")?
        .context("No focused object")?;
    let object = edit_section.object(&obj);
    let mut alias = object.get_alias().context("get_alias failed")?;
    let effect_count = object
        .count_effect(effect_name)
        .context("count_effect failed")?;

    if effect_count == 0 {
        let effect_name =
            get_effect_name(&alias).ok_or_else(|| anyhow::anyhow!("effect name not found"))?;

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
        alias.push_str(&effect_alias);

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

pub struct EguiApp {
    pub _handle: AviUtl2EframeHandle,
    app_config: AppConfig,
}

impl EguiApp {
    pub fn new(cc: &eframe::CreationContext<'_>, frame_handle: AviUtl2EframeHandle) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "NotoSansJP".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/NotoSansJP-Regular.ttf"))
                .into(),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "NotoSansJP".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.all_styles_mut(|style| {
            style.visuals = aviutl2_eframe::aviutl2_visuals();
        });

        egui_extras::install_image_loaders(&cc.egui_ctx);

        // 設定を読み込む
        let mut data_path = aviutl2::config::app_data_path();
        data_path.push(AppConfig::SETTINGS_FILE_PATH);
        let settings = match read_settings(data_path.to_str().unwrap()) {
            Ok(s) => s,
            Err(_) => crate::settings::Settings::new(),
        };
        let mut app_config = AppConfig::new();
        app_config.settings = settings;
        app_config.settings.button_scale = app_config
            .settings
            .button_scale
            .clamp(UiConfig::BUTTON_SCALE_MIN, UiConfig::BUTTON_SCALE_MAX);

        let button_w =
            UiConfig::COORD_BUTTON_SIZE[0] * (app_config.settings.button_scale as f32 / 100.0);
        let button_h =
            UiConfig::COORD_BUTTON_SIZE[1] * (app_config.settings.button_scale as f32 / 100.0);
        app_config.coord_button_size = [button_w, button_h];

        Self {
            _handle: frame_handle,
            app_config,
        }
    }

    pub fn render_setting_modal(&mut self, ui: &mut egui::Ui) {
        let width = f32::max(0.0, ui.available_width() - UiConfig::MODAL_PADDING * 2.0);
        let _modal = egui::Modal::new(egui::Id::new("Settings Modal")).show(ui.ctx(), |ui| {
            ui.set_width(width);
            ui.label(tr("設定"));

            let _checkbox_res = ui.checkbox(
                &mut self.app_config.settings.reset_offset,
                tr("変更時にオフセットをリセット"),
            );

            let slider_text = tr("ボタンサイズ");
            let text_width = ui
                .painter()
                .layout_no_wrap(
                    slider_text.to_owned(),
                    ui.style()
                        .text_styles
                        .get(&egui::TextStyle::Body)
                        .unwrap()
                        .clone(),
                    egui::Color32::WHITE,
                )
                .size()
                .x;

            ui.scope(|ui| {
                // スライダーをモーダルウィンドウに収めるために幅を調整
                ui.spacing_mut().slider_width = f32::max(
                    0.0,
                    width
                        - ui.spacing().interact_size.x
                        - ui.spacing().button_padding.x
                        - text_width,
                );
                ui.add(
                    egui::Slider::new(
                        &mut self.app_config.settings.button_scale,
                        UiConfig::BUTTON_SCALE_MIN..=UiConfig::BUTTON_SCALE_MAX,
                    )
                    .text(slider_text),
                );
            });

            let button_w = UiConfig::COORD_BUTTON_SIZE[0]
                * (self.app_config.settings.button_scale as f32 / 100.0);
            let button_h = UiConfig::COORD_BUTTON_SIZE[1]
                * (self.app_config.settings.button_scale as f32 / 100.0);
            self.app_config.coord_button_size = [button_w, button_h];

            if ui.button(tr("閉じる")).clicked() {
                self.app_config.show_settings_window = false;
                // モーダルウィンドウを閉じるときに設定を書き込む
                let mut data_path = aviutl2::config::app_data_path();
                data_path.push(AppConfig::SETTINGS_FILE_PATH);
                if let Err(e) =
                    write_settings(data_path.to_str().unwrap(), &self.app_config.settings)
                {
                    aviutl2::lprintln!(error, "{}", e);
                }
            }
        });
    }
}

impl eframe::App for EguiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let image_color = aviutl2_eframe::aviutl2_visuals().text_color();

        // ヘッダー
        let _ = if !self.app_config.is_header_expanded {
            (|| {
                let header = egui::Panel::top("collapsed_header")
                    .exact_size(UiConfig::HEADER_COLLAPSED_HEIGHT)
                    .show_inside(ui, |_ui| {});

                let response = header.response;
                if response.hovered() {
                    let hover_color = egui::Color32::from_white_alpha(32);
                    response.ctx.layer_painter(response.layer_id).rect_filled(
                        response.rect,
                        0.0,
                        hover_color,
                    );
                }
                if response.interact(egui::Sense::click()).clicked() {
                    self.app_config.is_header_expanded = !self.app_config.is_header_expanded;
                }
            })()
        } else {
            (|| {
                egui::Panel::top("header_content")
                .show_animated_inside(ui, self.app_config.is_header_expanded, |ui| {
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.x = UiConfig::HEADER_ITEM_SPACING;
                        ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let button_size = egui::Vec2::new(UiConfig::HEADER_BUTTON_SIZE[0], UiConfig::HEADER_BUTTON_SIZE[1]);

                            if ui.add_sized(
                                    button_size,
                                    egui::Button::image(
                                        egui::Image::new(icon!("material-symbols:settings-outline-rounded", color = "white"))
                                            .fit_to_exact_size(button_size).tint(image_color)).fill(egui::Color32::TRANSPARENT),
                                )
                                .clicked()
                            {
                                self.app_config.show_settings_window = true;
                            }

                            if ui.add_sized(
                                    button_size,
                                    egui::Button::image(
                                        egui::Image::new(icon!("material-symbols:keyboard-arrow-up-rounded", color = "white"))
                                            .fit_to_exact_size(button_size).tint(image_color),
                                    ).fill(egui::Color32::TRANSPARENT),
                                )
                                .clicked()
                            {
                                self.app_config.is_header_expanded = !self.app_config.is_header_expanded;
                            }
                        });
                    });
                });
            });
            })()
        };

        if self.app_config.show_settings_window {
            self.render_setting_modal(ui);
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.scope(|ui| {
                let button_size = self.app_config.coord_button_size;
                ui.style_mut().spacing.button_padding = egui::vec2(
                    UiConfig::COORD_BUTTON_PADDING[0],
                    UiConfig::COORD_BUTTON_PADDING[1],
                );
                egui::Grid::new("setting_grid")
                    .min_col_width(UiConfig::MIN_COL_WIDTH)
                    .min_row_height(UiConfig::MIN_COL_HEIGHT)
                    .spacing(UiConfig::GRID_SPACING)
                    .show(ui, |ui| {
                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/corner.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::LeftTop;
                        }
                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/side.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color)
                                    .rotate(std::f32::consts::FRAC_PI_2, egui::Vec2::splat(0.5)),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::Top;
                        }
                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/corner.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color)
                                    .rotate(std::f32::consts::FRAC_PI_2, egui::Vec2::splat(0.5)),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::RightTop;
                        }
                        ui.end_row();

                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/side.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::Left;
                        }
                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/center.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::Center;
                        }
                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/side.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color)
                                    .rotate(std::f32::consts::PI, egui::Vec2::splat(0.5)),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::Right;
                        }
                        ui.end_row();

                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/corner.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color)
                                    .rotate(-std::f32::consts::FRAC_PI_2, egui::Vec2::splat(0.5)),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::LeftBottom;
                        }
                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/side.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color)
                                    .rotate(-std::f32::consts::FRAC_PI_2, egui::Vec2::splat(0.5)),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::Bottom;
                        }
                        if ui
                            .add_sized(
                                button_size,
                                egui::Button::image(
                                    egui::Image::new(egui::include_image!(
                                        "../assets/icons/corner.svg"
                                    ))
                                    .fit_to_exact_size(egui::Vec2::new(
                                        button_size[0],
                                        button_size[1],
                                    ))
                                    .tint(image_color)
                                    .rotate(std::f32::consts::PI, egui::Vec2::splat(0.5)),
                                ),
                            )
                            .clicked()
                        {
                            self.app_config.is_coord_button_clicked = true;
                            self.app_config.coord = Coord::RightBottom;
                        }
                        ui.end_row();
                    });
            });

            if self.app_config.is_coord_button_clicked {
                self.app_config.is_coord_button_clicked = false;
                if let Err(e) = crate::EDIT_HANDLE.call_edit_section(
                    |edit_section: &mut aviutl2::generic::EditSection| {
                        let result: Result<()> = (|| {
                            ensure_effect(
                                edit_section,
                                AppConfig::EFFECT_NAME,
                                AppConfig::EFFECT_ALIAS,
                            )
                            .context("ensure_effect failed")?;

                            let obj = edit_section
                                .get_focused_object()
                                .context("get_focused_object failed")?
                                .context("No focused object")?;
                            let object = edit_section.object(&obj);
                            let effect_count = object
                                .count_effect(AppConfig::EFFECT_NAME)
                                .context("count_effect failed")?;

                            object
                                .set_effect_item(
                                    AppConfig::EFFECT_NAME,
                                    effect_count - 1,
                                    "位置",
                                    self.app_config.coord.as_str(),
                                )
                                .context("set_effect_item failed")?;

                            // 中心座標変更時にオフセットをリセットする場合
                            if self.app_config.settings.reset_offset {
                                object
                                    .set_effect_item(
                                        AppConfig::EFFECT_NAME,
                                        effect_count - 1,
                                        "オフセットX",
                                        "0",
                                    )
                                    .context("set_effect_item failed")?;
                                object
                                    .set_effect_item(
                                        AppConfig::EFFECT_NAME,
                                        effect_count - 1,
                                        "オフセットY",
                                        "0",
                                    )
                                    .context("set_effect_item failed")?;
                            }

                            Ok(())
                        })();

                        if let Err(e) = result {
                            aviutl2::lprintln!(error, "error: {:?}", e);
                        }
                    },
                ) {
                    aviutl2::lprintln!(error, "edit handle error: {:?}", e);
                };
            }
        });
    }
}
