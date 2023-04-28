use anyhow::{anyhow, Error};
use eframe::egui;
use g_translator_m::pasring_and_translate;

#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::rc::Rc;

const PADDING_NARROW: f32 = 3.0;
const PADDING_WIDE: f32 = 10.0;
const WIDTH_RATIO: f32 = 0.5;

#[derive(Debug)]
pub struct BigFrame {
    items: Vec<String>,
    character_item: CharacterItem,
    #[cfg(not(target_arch = "wasm32"))]
    file_path: Option<std::path::PathBuf>,
    #[cfg(target_arch = "wasm32")]
    file_content: Rc<RefCell<Option<(Vec<u8>, String)>>>,
    #[cfg(not(target_arch = "wasm32"))]
    runtime: tokio::runtime::Runtime,
    etc_value: EtcValue,
    receiver: Receiver,
}

#[derive(Debug)]
struct Receiver {
    translation_rx: Option<std::sync::mpsc::Receiver<Vec<String>>>,
    download_link_rx: Option<std::sync::mpsc::Receiver<String>>,
    #[cfg(target_arch = "wasm32")]
    file_rx: Option<std::sync::mpsc::Receiver<(Vec<u8>, String)>>,
}

#[derive(Default, Debug)]
struct CharacterItem {
    is_korean: bool,
    file_name: String,
    creator: String,
    character_name: String,
    tags: String,
    download_link: String,
    note: String,
    korean_description: String,
    english_description: String,
    category: String,
}

#[derive(Debug, Default)]
struct EtcValue {
    auto_translation: bool,
    auto_download_link: bool,
    making_translation: bool,
    making_download_link: bool,
}

impl BigFrame {
    pub fn _new(cc: &eframe::CreationContext<'_>) -> Self {
        _setup_custom_font(&cc.egui_ctx);
        let strings = [
            "파일명/Image: Yuzu".to_string(),
            "제작자/Creator: 제작자".to_string(),
            "이름/Name: 유즈 / Yuzu".to_string(),
            "태그/Tags: [[고양이수인(Catgirl)]], [[여성(female)]], [[메이드(maid)]]".to_string(),
            "Download link: https://catbox.moe".to_string(),
            "비고/Note: 수줍은 고양이 소녀 메이드 / Shy Cat Girl Maid".to_string(),
            "한글 설명".to_string(),
            "English description".to_string(),
            "분류: [[분류:소녀(Girl)]] [[분류:메이드(Maid)]] [[분류:고양이수인(Catgirl)]]"
                .to_string(),
        ];
        let items = Vec::from(strings);
        let character_item = CharacterItem::default();
        #[cfg(not(target_arch = "wasm32"))]
        let file_path = None;
        #[cfg(not(target_arch = "wasm32"))]
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let etc_value = EtcValue::default();
        Self {
            items,
            character_item,
            #[cfg(not(target_arch = "wasm32"))]
            file_path,
            #[cfg(target_arch = "wasm32")]
            file_content: Rc::new(RefCell::new(None)),
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
            etc_value,
            receiver: Receiver {
                translation_rx: None,
                download_link_rx: None,
                #[cfg(target_arch = "wasm32")]
                file_rx: None,
            },
        }
    }

    fn clear_fields(&mut self) {
        self.character_item.file_name = String::new();
        self.character_item.creator = String::new();
        self.character_item.character_name = String::new();
        self.character_item.tags = String::new();
        self.character_item.download_link = String::new();
        self.character_item.note = String::new();
        self.character_item.korean_description = String::new();
        self.character_item.english_description = String::new();
        self.character_item.category = String::new();
    }

    fn parsing_png(&mut self) -> Result<[String; 4], Error> {
        use png_parser::{check_vaild, parsing_text, parsing_text_for_cat, read_chunks, Character};

        #[cfg(not(target_arch = "wasm32"))]
        let file_data = read_file_to_vec(self.file_path.as_ref().unwrap())?;
        #[cfg(not(target_arch = "wasm32"))] 
        let file_data = file_data.as_slice();

        #[cfg(target_arch = "wasm32")]
        let a = self.file_content.borrow();
        #[cfg(target_arch = "wasm32")]
        let file = &a.as_ref().unwrap().0;
        #[cfg(target_arch = "wasm32")]
        let file_data = file.as_slice();

        let vec_chunks =
            read_chunks(file_data).map_err(|_| anyhow!("유효하지 않은 캐릭터 카드입니다."))?;
        check_vaild(&vec_chunks).map_err(|_| anyhow!("유효하지 않은 캐릭터 카드입니다."))?;
        let script = parsing_text(vec_chunks);

        // "tEXt" 가 없으면 에러
        if script.is_none() {
            return Err(anyhow!("유효하지 않은 캐릭터 카드입니다."));
        }

        let character: Character = serde_json::from_str(script.unwrap().as_str())?;
        let (character_name, note, description) = parsing_text_for_cat(character);
        let (mut korean_description, mut english_description) = (String::new(), String::new());
        if character_name.chars().all(|c| c.is_ascii()) {
            self.character_item.is_korean = false;
            english_description = description;
        } else {
            self.character_item.is_korean = true;
            korean_description = description;
        }
        Ok([
            character_name,
            note,
            korean_description,
            english_description,
        ])
    }

    // Binding parsed data to variables
    fn binding(&mut self) -> Result<[String; 4], Error> {
        // let [a, b, c, d] = self.parsing_png();
        let [a, b, c, d] = match self.parsing_png() {
            Ok(strings) => strings,
            Err(e) => return Err(e),
        };
        self.character_item.character_name = a.clone();
        self.character_item.note = b.clone();
        self.character_item.korean_description = c.clone();
        self.character_item.english_description = d.clone();

        Ok([a, b, c, d])
    }

    fn updating_translated_data(&mut self) {
        if let Some(data_rx) = &self.receiver.translation_rx {
            match data_rx.try_recv() {
                Ok(translated) => match translated[0].as_str() {
                    x if x == "name" && !self.character_item.is_korean => {
                        self.character_item.character_name =
                            format!("{} / {}", translated[1].clone(), translated[2].clone());
                    }
                    x if x == "name" && self.character_item.is_korean => {
                        self.character_item.character_name =
                            format!("{} / {}", translated[2].clone(), translated[1].clone());
                    }
                    x if x == "note" && !self.character_item.is_korean => {
                        self.character_item.note =
                            format!("{} / {}", translated[1].clone(), translated[2].clone())
                    }
                    x if x == "note" && !self.character_item.is_korean => {
                        self.character_item.note =
                            format!("{} / {}", translated[2].clone(), translated[1].clone())
                    }
                    x if x == "desc" && self.character_item.is_korean => {
                        self.character_item.english_description = translated[1].clone()
                    }
                    x if x == "desc" && !self.character_item.is_korean => {
                        self.character_item.korean_description = translated[1].clone()
                    }
                    _ => (),
                },
                Err(error) => match error {
                    std::sync::mpsc::TryRecvError::Empty => {
                        // eprintln!("Error. Translation channel is empty.")
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        self.receiver.translation_rx.take();
                        self.etc_value.making_translation = false;
                    }
                },
            }
        }
    }

    fn updating_download_link(&mut self) {
        if let Some(link_rx) = &self.receiver.download_link_rx {
            match link_rx.try_recv() {
                Ok(link) => {
                    self.character_item.download_link = link;
                }
                Err(error) => match error {
                    std::sync::mpsc::TryRecvError::Empty => {
                        // eprintln!("Error. Download link channel is empty.")
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        self.receiver.download_link_rx.take();
                        self.etc_value.making_download_link = false;
                    }
                },
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn updating_file(&mut self) {
        if let Some(file_rx) = &self.receiver.file_rx {
            match file_rx.try_recv() {
                Ok(file) => {
                    {
                        let mut a = self.file_content.borrow_mut();
                        *a = Some(file);
                    }
                    if let Err(error) = self.all_processing() {
                        eprintln!("{error}");
                    }
                }
                Err(error) => match error {
                    std::sync::mpsc::TryRecvError::Empty => {
                        // eprintln!("Error. File channel is empty.")
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => {
                        self.receiver.file_rx.take();
                    }
                },
            }
        }
    }

    fn all_processing(&mut self) -> Result<(), Error> {
        let mut vecs = vec![];
        match self.binding() {
            Ok(strings) => strings.into_iter().for_each(|a| vecs.push(a)),
            Err(e) => {
                return Err(e);
            }
        }

        let english_d = vecs.pop().unwrap();
        let korean_d = vecs.pop().unwrap();
        let chara_note = vecs.pop().unwrap();
        let chara_name = vecs.pop().unwrap();

        if self.etc_value.auto_translation {
            let (tx, rx) = std::sync::mpsc::channel();
            self.etc_value.making_translation = true;
            let from = if self.character_item.is_korean {
                "ko"
            } else {
                "en"
            };
            let to = if self.character_item.is_korean {
                "en"
            } else {
                "ko"
            };
            self.receiver.translation_rx = Some(rx);
            let is_korean = self.character_item.is_korean;

            #[cfg(not(target_arch = "wasm32"))]
            self.runtime.spawn(async move {
                let t_name = translate_name(tx.clone(), chara_name, from, to);
                let t_note = translate_note(tx.clone(), chara_note, from, to);
                let t_d = if !is_korean {
                    translate_d(tx, english_d, from, to)
                } else {
                    translate_d(tx, korean_d, from, to)
                };

                tokio::join!(t_name, t_note, t_d);
            });

            #[cfg(target_arch = "wasm32")]
            wasm_bindgen_futures::spawn_local(async move {
                let t_name = translate_name(tx.clone(), chara_name, from, to);
                let t_note = translate_note(tx.clone(), chara_note, from, to);
                let t_d = if !is_korean {
                    translate_d(tx, english_d, from, to)
                } else {
                    translate_d(tx, korean_d, from, to)
                };

                futures::join!(t_name, t_note, t_d);
            });
        }

        if self.etc_value.auto_download_link {
            let (download_tx, download_rx) = std::sync::mpsc::channel();
            self.etc_value.making_download_link = true;
            self.receiver.download_link_rx = Some(download_rx);

            #[cfg(not(target_arch = "wasm32"))]
            {
                let file_path = self.file_path.clone().unwrap();
                let file_path = file_path.to_str().unwrap().to_owned();

                self.runtime.spawn(async move {
                    if let Ok(link) = catbox::file::from_file(file_path.as_str(), None)
                        .await
                        .map_err(|_| anyhow!("Failed to create the download link"))
                    {
                        // std::thread::sleep(std::time::Duration::from_secs(2));
                        // let link = "text.com".to_string();
                        if let Err(e) = download_tx.send(link) {
                            eprintln!("Failed to send the download link to receiver...{e}");
                        }
                        println!("Creating download link complete");
                    }
                });
            }

            #[cfg(target_arch = "wasm32")]
            {
                let a = self.file_content.borrow();
                let file = a.as_ref().unwrap().clone();

                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(link) = catbox_wasm::upload_file(file.0, file.1)
                        .await
                        .map_err(|_| anyhow!("Failed to create the download link"))
                    {
                        // let link = "text.com".to_string();
                        if let Err(e) = download_tx.send(link) {
                            eprintln!("Failed to send the download link to receiver...{e}");
                        }
                        println!("Creating download link complete");
                    }
                });
            }
        }

        Ok(())
    }

    fn render_leftside(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Drag & drop a image file")
                        .text_style(egui::TextStyle::Heading),
                );
                // open file
                if ui.button("open file...").clicked() {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("card", &["png"])
                            .pick_file()
                        {
                            self.clear_fields();
                            self.file_path = Some(path);
                            if let Err(error) = self.all_processing() {
                                eprintln!("{error}");
                            }
                        }
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        let (file_tx, file_rx) = std::sync::mpsc::channel();
                        self.receiver.file_rx = Some(file_rx);
                        let task = rfd::AsyncFileDialog::new()
                            .add_filter("card", &["png"])
                            .pick_file();
                        wasm_bindgen_futures::spawn_local(async move {
                            let file = task.await;

                            if let Some(file) = file {
                                let file_name = file.file_name().clone();
                                let file = file.read().await;
                                file_tx.send((file, file_name));
                            }
                        });
                    }
                }
            });
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::RIGHT), |ui| {
                ui.checkbox(&mut self.etc_value.auto_translation, "구글 번역 사용");
                ui.checkbox(
                    &mut self.etc_value.auto_download_link,
                    "다운로드 링크 자동 생성",
                );
            });
        });

        preview_files_being_dropped(ctx);

        // Collect dropped files && translate
        #[cfg(not(target_arch = "wasm32"))]
        ctx.input(|i| {
            let a = i
                .raw
                .dropped_files
                .iter()
                .map(|f| {
                    f.path
                        .as_ref()
                        .unwrap()
                        .extension()
                        .unwrap()
                        .to_str()
                        .unwrap()
                })
                .all(|e| e == "png");
            if !i.raw.dropped_files.is_empty() && a {
                self.clear_fields();
                self.file_path = i.raw.dropped_files[0].clone().path;
                if let Err(error) = self.all_processing() {
                    eprintln!("{error}");
                }
            }
        });

        #[cfg(target_arch = "wasm32")]
        ctx.input(|i| {
            let a = i
                .raw
                .dropped_files
                .iter()
                .map(|f| f.name.ends_with(".png"))
                .all(|x| x);

            if !i.raw.dropped_files.is_empty() && a {
                self.clear_fields();
                let file = i.raw.dropped_files[0].clone();
                let file_name = file.name;
                let file_data = file.bytes;
                if let Some(file_data) = file_data {
                    let file_data = file_data.to_vec();
                    let mut a = self.file_content.borrow_mut();
                    *a = Some((file_data, file_name));
                }

                if let Err(error) = self.all_processing() {
                    eprintln!("{error}");
                }
            }
        });

        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.receiver.translation_rx.is_some() || self.receiver.download_link_rx.is_some() {
                ctx.request_repaint();
            };
        }

        #[cfg(target_arch = "wasm32")]
        {
            if self.receiver.translation_rx.is_some()
                || self.receiver.download_link_rx.is_some()
                || self.receiver.file_rx.is_some()
            {
                ctx.request_repaint();
            };
        }

        self.updating_translated_data();
        self.updating_download_link();
        #[cfg(target_arch = "wasm32")]
        self.updating_file();

        let mut name_arr = [
            &mut self.character_item.file_name,
            &mut self.character_item.creator,
            &mut self.character_item.character_name,
            &mut self.character_item.tags,
            &mut self.character_item.download_link,
            &mut self.character_item.note,
            &mut self.character_item.korean_description,
            &mut self.character_item.english_description,
            &mut self.character_item.category,
        ];
        let mut items_iter = self.items.iter();

        ui.add_space(PADDING_WIDE);
        let screen_width = ctx.available_rect().width();
        items_iter.by_ref().take(6).enumerate().for_each(|(i, a)| {
            let text = a.split(": ").next().unwrap();
            let hint = a.split(": ").nth(1).unwrap();
            ui.label(egui::RichText::new(text).text_style(egui::TextStyle::Heading));
            ui.add_space(PADDING_NARROW);
            if (self.etc_value.making_translation && (i == 2 || i == 5))
                || (self.etc_value.making_download_link && i == 4)
            {
                let a = &mut name_arr[i].as_str();
                ui.add(
                    egui::TextEdit::singleline(a)
                        .hint_text(hint)
                        .margin(egui::vec2(10., 10.)), // .desired_width(screen_width),
                );
            } else {
                ui.add(
                    egui::TextEdit::singleline(name_arr[i])
                        .hint_text(hint)
                        .margin(egui::vec2(10., 10.))
                        .desired_width(screen_width),
                );
            }
            ui.add_space(PADDING_WIDE);
        });

        items_iter.by_ref().take(2).enumerate().for_each(|(i, a)| {
            egui::containers::Resize::default()
                .fixed_size([screen_width * WIDTH_RATIO, 100.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(a).text_style(egui::TextStyle::Heading));
                    ui.add_space(PADDING_NARROW);
                    egui::ScrollArea::vertical()
                        .id_source(format!("{}", i).as_str())
                        .show(ui, |ui| {
                            if self.etc_value.making_translation {
                                let a = &mut name_arr[i + 6].as_str();
                                let text_edit = egui::TextEdit::multiline(a)
                                    .margin(egui::vec2(10., 10.))
                                    .desired_rows(4)
                                    .desired_width(screen_width);
                                ui.add(text_edit);
                            } else {
                                let text_edit = egui::TextEdit::multiline(name_arr[i + 6])
                                    .margin(egui::vec2(10., 10.))
                                    .desired_rows(4)
                                    .desired_width(screen_width);
                                ui.add(text_edit);
                            }
                        });
                    ui.add_space(PADDING_WIDE);
                })
        });

        items_iter.by_ref().take(1).for_each(|a| {
            let text = a.split(": ").next().unwrap();
            let hint = a.split(": ").nth(1).unwrap();
            ui.label(egui::RichText::new(text).text_style(egui::TextStyle::Heading));
            ui.add_space(PADDING_NARROW);
            ui.add(
                egui::TextEdit::singleline(name_arr[name_arr.len() - 1])
                    .hint_text(hint)
                    .margin(egui::vec2(10., 10.))
                    .desired_width(screen_width),
            );
        });
    }

    fn render_central(&self, ctx: &egui::Context) {
        let result = format!(
            "\
||<width=15%>이미지||<width=50%>[[파일:{}.png|align=center]]||
||<width=15%>제작자 / Creator||<width=85%>{}||
||<width=15%>이름 / Name||<width=85%>{}||
||<width=15%>태그 / Tags||<width=85%>{}||
||<width=15%>Download link||<width=85%>[[{}]]||
||<width=15%>비고 / Note||<width=85%>{}||
||<width=15%>한글 설명||<width=85%>{}||
||<width=15%>English Description||<width=85%>{}||
{}",
            self.character_item.file_name,
            self.character_item.creator,
            self.character_item.character_name,
            self.character_item.tags,
            self.character_item.download_link,
            self.character_item.note,
            self.character_item.korean_description,
            self.character_item.english_description,
            self.character_item.category
        );
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if ui
                    .add_sized(
                        egui::vec2(ctx.available_rect().width() * 0.97, 50.0),
                        egui::Button::new("Copy"),
                    )
                    .clicked()
                {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        ui.output_mut(|o| o.copied_text = result.clone());
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        let window = web_sys::window().unwrap();
                        let clipboard: web_sys::Clipboard = window.navigator().clipboard().unwrap();
                        clipboard.write_text(&result);
                    }
                }
                ui.add_space(PADDING_WIDE * 2.0);
                ui.add(
                    egui::TextEdit::multiline(&mut result.as_str())
                        .desired_width(ctx.available_rect().width()),
                )
            });
        });
    }
}

impl eframe::App for BigFrame {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let screen_width = ctx.available_rect().width();

        egui::SidePanel::left("lefe_panel")
            .min_width(screen_width * 0.2)
            .max_width(screen_width * 0.8)
            .resizable(false)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_leftside(ui, ctx);
                })
            });

        self.render_central(ctx);
    }
}

fn _setup_custom_font(ctx: &egui::Context) {
    // start default fonts
    let mut fonts = egui::FontDefinitions::default();

    // install font
    fonts.font_data.insert(
        "NanumGothic".to_string(),
        egui::FontData::from_static(include_bytes!("../font/NanumGothic.ttf")),
    );

    // put my font first, proportional
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "NanumGothic".to_string());

    // put my font last, monospace
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("NanumGothic".to_string());

    // set font
    ctx.set_fonts(fonts);
}

fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::*;

    if ctx.input(|i| {
        #[cfg(not(target_arch = "wasm32"))]
        {
            !i.raw.hovered_files.is_empty()
                && i.raw
                    .hovered_files
                    .iter()
                    .map(|f| {
                        f.path
                            .as_ref()
                            .unwrap()
                            .extension()
                            .unwrap()
                            .to_str()
                            .unwrap()
                    })
                    .all(|e| e == "png")
        }
        #[cfg(target_arch = "wasm32")]
        {
            !i.raw.hovered_files.is_empty()
                && i.raw
                    .hovered_files
                    .iter()
                    .map(|f| f.mime == "image/png")
                    .all(|x| x)
        }
    }) {
        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
    }
}

fn _config_text_styles(ctx: &egui::Context) {
    use egui::FontFamily::Proportional;
    use egui::TextStyle;

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, egui::FontId::new(22.0, Proportional)),
        (TextStyle::Body, egui::FontId::new(22.0, Proportional)),
    ]
    .into();
    ctx.set_style(style);
}

async fn translate_name(
    tx: std::sync::mpsc::Sender<Vec<String>>,
    input: String,
    from: &str,
    to: &str,
) {
    let input_original = input.clone();
    if let Ok(translated) = pasring_and_translate(input, from, to).await {
        if let Err(e) = tx.send(vec!["name".to_string(), translated, input_original]) {
            eprintln!("Error sending translated data...{e}");
        }
        println!("name translation complete");
    }
}

async fn translate_note(
    tx: std::sync::mpsc::Sender<Vec<String>>,
    input: String,
    from: &str,
    to: &str,
) {
    let input_original = input.clone();
    if let Ok(translated) = pasring_and_translate(input, from, to).await {
        if let Err(e) = tx.send(vec!["note".to_string(), translated, input_original]) {
            eprintln!("Error sending translated data...{e}");
        }
        println!("note translation complete");
    }
}

async fn translate_d(
    tx: std::sync::mpsc::Sender<Vec<String>>,
    input: String,
    from: &str,
    to: &str,
) {
    let input_count = input.chars().count();
    match input_count {
        x if x < 1900 => {
            if let Ok(translated) = pasring_and_translate(input, from, to).await {
                if let Err(e) = tx.send(vec!["desc".to_string(), translated]) {
                    eprintln!("Error sending translated data...{e}");
                }
                println!("description translation complete");
            }
        }
        x if x >= 1900 => {
            // 1900글자씩 자름
            let input_lines = input.lines().collect::<Vec<_>>();
            let mut count = 0;
            let mut vecs_line = vec![];
            let mut translated_result = vec![];

            for line in input_lines {
                count += line.chars().count();
                println!("{count}: {line}");
                if count >= 1900 {
                    // 그냥 "\n" 이라고만 했더니 조금 작동이 이상해서 "\\\n"으로 구분하고 나중에 "\n"으로 일괄 바꾸기로 함
                    let input = vecs_line.join("\\\n");
                    // let input_c = input.clone();
                    if let Ok(translated) = pasring_and_translate(input, from, to).await {
                        let translated = translated.replace("\\\n", "\n");
                        // println!("{}", translated);
                        // println!("{}", input_c);
                        translated_result.push(translated);
                        vecs_line = vec![];
                        count = line.chars().count();
                    }
                }
                vecs_line.push(line);
            }

            let input = vecs_line.join("\\\n");
            if let Ok(translated) = pasring_and_translate(input, from, to).await {
                let translated = translated.replace("\\\n", "\n");
                translated_result.push(translated);
            }

            let translated = translated_result.join("\n");

            if let Err(e) = tx.send(vec!["desc".to_string(), translated]) {
                eprintln!("Error sending translated data...{e}");
            } else {
                println!("description translation complete");
            }
        }
        _ => (),
    }
}

fn read_file_to_vec(path: &std::path::PathBuf) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}
