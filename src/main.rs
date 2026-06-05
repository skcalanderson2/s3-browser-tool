#![allow(clippy::result_large_err)]

use std::sync::Arc;

use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::{Client, config::Region};
use iced::{
    Alignment, Color, Element, Length, Point, Subscription, Task, Theme,
    widget::{
        Space, button, column, container, mouse_area, row, scrollable, stack, text, text_input,
    },
};

fn main() -> iced::Result {
    dotenvy::dotenv().ok();
    iced::application("S3 Browser Tool", App::update, App::view)
        .theme(|_| Theme::Dark)
        .subscription(|app: &App| app.subscription())
        .run_with(App::new)
}

// Newtype so Arc<Client> is Debug + Clone for Message derive
#[derive(Clone)]
struct S3Client(Arc<Client>);

impl std::fmt::Debug for S3Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Client").finish()
    }
}

struct App {
    bucket: String,
    files: Vec<String>,
    status: String,
    client: Option<S3Client>,
    loading: bool,
    mouse_pos: Point,
    context_menu: Option<ContextMenu>,
    dialog: Option<Dialog>,
}

struct ContextMenu {
    key: String,
    pos: Point,
}

enum Dialog {
    Upload { path: String },
    ConfirmDelete { key: String },
}

#[derive(Debug, Clone)]
enum Message {
    BucketChanged(String),
    Connect,
    ClientReady(S3Client),
    Refresh,
    FilesLoaded(Result<Vec<String>, String>),
    FileRightClicked(String),
    CloseContextMenu,
    ShowUploadDialog,
    PickFile,
    FilePicked(Option<String>),
    UploadPathChanged(String),
    ConfirmUpload,
    FileUploaded(Result<(), String>),
    ConfirmDeleteFile(String),
    DeleteConfirmed,
    FileDeleted(Result<(), String>),
    CancelDialog,
    MouseMoved(Point),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let bucket = std::env::var("AWS_S3_BUCKET").unwrap_or_default();
        let app = App {
            bucket,
            files: Vec::new(),
            status: "Initializing...".into(),
            client: None,
            loading: false,
            mouse_pos: Point::ORIGIN,
            context_menu: None,
            dialog: None,
        };
        let task = Task::perform(
            async {
                let region_provider = RegionProviderChain::default_provider()
                    .or_else(Region::new("us-east-1"));
                let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
                    .region(region_provider)
                    .load()
                    .await;
                S3Client(Arc::new(Client::new(&config)))
            },
            Message::ClientReady,
        );
        (app, task)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::BucketChanged(s) => {
                self.bucket = s;
                Task::none()
            }

            Message::Connect | Message::Refresh => {
                let Some(client) = self.client.clone() else {
                    return Task::none();
                };
                if self.bucket.is_empty() {
                    return Task::none();
                }
                self.loading = true;
                self.context_menu = None;
                self.status = "Loading...".into();
                let bucket = self.bucket.clone();
                Task::perform(
                    async move {
                        s3_browser_tool::list_objects_keys(&client.0, &bucket)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::FilesLoaded,
                )
            }

            Message::ClientReady(client) => {
                self.client = Some(client.clone());
                self.status = "Ready".into();
                if !self.bucket.is_empty() {
                    self.update(Message::Refresh)
                } else {
                    Task::none()
                }
            }

            Message::FilesLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(files) => {
                        self.status = format!("{} objects", files.len());
                        self.files = files;
                    }
                    Err(e) => self.status = format!("Error: {e}"),
                }
                Task::none()
            }

            Message::FileRightClicked(key) => {
                self.context_menu = Some(ContextMenu {
                    key,
                    pos: self.mouse_pos,
                });
                Task::none()
            }

            Message::CloseContextMenu => {
                self.context_menu = None;
                Task::none()
            }

            Message::ShowUploadDialog => {
                self.context_menu = None;
                self.dialog = Some(Dialog::Upload { path: String::new() });
                Task::none()
            }

            Message::PickFile => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_file()
                        .await
                        .map(|f| f.path().to_string_lossy().into_owned())
                },
                Message::FilePicked,
            ),

            Message::FilePicked(Some(path)) => {
                self.dialog = Some(Dialog::Upload { path });
                Task::none()
            }

            Message::FilePicked(None) => Task::none(),

            Message::UploadPathChanged(path) => {
                if let Some(Dialog::Upload { path: p }) = &mut self.dialog {
                    *p = path;
                }
                Task::none()
            }

            Message::ConfirmUpload => {
                let Some(Dialog::Upload { path }) = self.dialog.take() else {
                    return Task::none();
                };
                let Some(client) = self.client.clone() else {
                    return Task::none();
                };
                let key = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&path)
                    .to_string();
                let bucket = self.bucket.clone();
                self.status = format!("Uploading {key}...");
                Task::perform(
                    async move {
                        s3_browser_tool::upload_object(&client.0, &bucket, &path, &key)
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                    Message::FileUploaded,
                )
            }

            Message::FileUploaded(Ok(())) => {
                self.status = "Upload complete".into();
                self.update(Message::Refresh)
            }

            Message::FileUploaded(Err(e)) => {
                self.status = format!("Upload error: {e}");
                Task::none()
            }

            Message::ConfirmDeleteFile(key) => {
                self.context_menu = None;
                self.dialog = Some(Dialog::ConfirmDelete { key });
                Task::none()
            }

            Message::DeleteConfirmed => {
                let Some(Dialog::ConfirmDelete { key }) = self.dialog.take() else {
                    return Task::none();
                };
                let Some(client) = self.client.clone() else {
                    return Task::none();
                };
                let bucket = self.bucket.clone();
                self.status = format!("Deleting {key}...");
                Task::perform(
                    async move {
                        s3_browser_tool::remove_object(&client.0, &bucket, &key)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::FileDeleted,
                )
            }

            Message::FileDeleted(Ok(())) => {
                self.status = "Deleted".into();
                self.update(Message::Refresh)
            }

            Message::FileDeleted(Err(e)) => {
                self.status = format!("Delete error: {e}");
                Task::none()
            }

            Message::CancelDialog => {
                self.dialog = None;
                Task::none()
            }

            Message::MouseMoved(pos) => {
                self.mouse_pos = pos;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let header = container(
            text("S3 Browser Tool").size(22).color(Color::WHITE),
        )
        .padding([14, 20])
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.07, 0.07, 0.12).into()),
            ..Default::default()
        });

        let toolbar = container(
            row![
                text("Bucket:").size(14),
                text_input("bucket name", &self.bucket)
                    .on_input(Message::BucketChanged)
                    .on_submit(Message::Connect)
                    .width(Length::Fixed(300.0)),
                button("Connect").on_press(Message::Connect),
                button("Refresh").on_press_maybe(
                    (!self.loading && self.client.is_some()).then_some(Message::Refresh),
                ),
                Space::with_width(Length::Fill),
                button("Upload File").on_press(Message::ShowUploadDialog),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([10, 20])
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.1, 0.1, 0.16).into()),
            ..Default::default()
        });

        let column_header = container(
            row![
                text("Key / File Name").size(12).color(Color::from_rgb(0.5, 0.5, 0.65)),
            ]
            .padding([4, 12]),
        )
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.09, 0.09, 0.14).into()),
            border: iced::Border {
                color: Color::from_rgb(0.2, 0.2, 0.3),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        let file_area: Element<Message> = if self.loading {
            container(
                text("Loading...").size(14).color(Color::from_rgb(0.5, 0.5, 0.65)),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else if self.files.is_empty() {
            container(
                text(if self.client.is_none() {
                    "Enter a bucket name and click Connect"
                } else {
                    "No objects in bucket"
                })
                .size(14)
                .color(Color::from_rgb(0.35, 0.35, 0.45)),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
        } else {
            let rows: Vec<Element<Message>> = self
                .files
                .iter()
                .enumerate()
                .map(|(i, key)| {
                    let k = key.clone();
                    let bg = if i % 2 == 0 {
                        Color::from_rgb(0.09, 0.09, 0.13)
                    } else {
                        Color::from_rgb(0.07, 0.07, 0.11)
                    };
                    mouse_area(
                        container(text(key.as_str()).size(13))
                            .padding([6, 12])
                            .width(Length::Fill)
                            .style(move |_| container::Style {
                                background: Some(bg.into()),
                                ..Default::default()
                            }),
                    )
                    .on_right_press(Message::FileRightClicked(k))
                    .into()
                })
                .collect();

            scrollable(column(rows).spacing(0))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        let file_panel = container(
            column![column_header, file_area].spacing(0),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.08, 0.08, 0.12).into()),
            ..Default::default()
        });

        let status_bar = container(
            text(&self.status)
                .size(12)
                .color(Color::from_rgb(0.5, 0.5, 0.6)),
        )
        .padding([5, 20])
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Color::from_rgb(0.06, 0.06, 0.1).into()),
            border: iced::Border {
                color: Color::from_rgb(0.15, 0.15, 0.25),
                width: 1.0,
                radius: 0.0.into(),
            },
            ..Default::default()
        });

        let base: Element<Message> =
            column![header, toolbar, file_panel, status_bar].into();

        // Dialog overlay
        if let Some(dialog) = &self.dialog {
            let dialog_widget: Element<Message> = match dialog {
                Dialog::Upload { path } => column![
                    text("Upload File").size(18),
                    text("Select a file to upload to the bucket.")
                        .size(13)
                        .color(Color::from_rgb(0.6, 0.6, 0.7)),
                    row![
                        text_input("File path...", path)
                            .on_input(Message::UploadPathChanged)
                            .width(Length::Fill),
                        button("Browse").on_press(Message::PickFile),
                    ]
                    .spacing(8),
                    row![
                        button("Upload").on_press(Message::ConfirmUpload),
                        button("Cancel").on_press(Message::CancelDialog),
                    ]
                    .spacing(8),
                ]
                .spacing(14)
                .padding(24)
                .into(),

                Dialog::ConfirmDelete { key } => column![
                    text("Confirm Delete").size(18),
                    text(format!("Delete '{key}'?"))
                        .size(14)
                        .color(Color::from_rgb(0.8, 0.6, 0.6)),
                    text("This cannot be undone.")
                        .size(12)
                        .color(Color::from_rgb(0.5, 0.4, 0.4)),
                    row![
                        button("Delete")
                            .style(button::danger)
                            .on_press(Message::DeleteConfirmed),
                        button("Cancel").on_press(Message::CancelDialog),
                    ]
                    .spacing(8),
                ]
                .spacing(14)
                .padding(24)
                .into(),
            };

            let dialog_box = container(dialog_widget)
                .width(Length::Fixed(450.0))
                .style(|_| container::Style {
                    background: Some(Color::from_rgb(0.13, 0.13, 0.21).into()),
                    border: iced::Border {
                        color: Color::from_rgb(0.3, 0.3, 0.5),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                });

            let backdrop = container(Space::new(Length::Fill, Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 }.into()),
                    ..Default::default()
                });

            return stack![
                base,
                backdrop,
                container(dialog_box)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            ]
            .into();
        }

        // Context menu overlay
        if let Some(ctx) = &self.context_menu {
            let menu = container(
                column![
                    button(text("Delete").size(13))
                        .style(button::danger)
                        .width(Length::Fill)
                        .on_press(Message::ConfirmDeleteFile(ctx.key.clone())),
                ]
                .spacing(2),
            )
            .padding(6)
            .width(Length::Fixed(160.0))
            .style(|_| container::Style {
                background: Some(Color::from_rgb(0.16, 0.16, 0.26).into()),
                border: iced::Border {
                    color: Color::from_rgb(0.35, 0.35, 0.55),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

            let x = ctx.pos.x;
            let y = ctx.pos.y;

            let positioned: Element<Message> = column![
                Space::with_height(Length::Fixed(y)),
                row![Space::with_width(Length::Fixed(x)), menu],
            ]
            .into();

            let dismiss = mouse_area(
                container(Space::new(Length::Fill, Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .on_press(Message::CloseContextMenu)
            .on_right_press(Message::CloseContextMenu);

            return stack![base, dismiss, positioned].into();
        }

        base
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _status, _window| {
            if let iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) = event {
                Some(Message::MouseMoved(position))
            } else {
                None
            }
        })
    }
}
