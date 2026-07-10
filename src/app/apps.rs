//! This modules handles the logic for each "app" that rustcast can load
//!
//! An "app" is effectively, one of the results that rustcast returns when you search for something

use std::io::Cursor;

use iced::{
    Alignment, Background, Color, Element,
    Length::{self, Fill},
    border::Radius,
    widget::{
        Button, Row, Text, container,
        image::{Handle, Viewer},
        space,
        text::Wrapping,
    },
};

use crate::{
    app::{Message, Page, RUSTCAST_DESC_NAME},
    clipboard::ClipBoardContentType,
    commands::Function,
    styles::{
        clipboard_icon, emoji_icon, favourite_button_style, filesearch_icon, info_icon, quit_icon,
        refresh_icon, result_button_style, result_row_container_style, settings_icon,
    },
    utils::icns_data_to_handle,
};

/// The rustcast icns icons bytes
pub const ICNS_ICON: &[u8] = include_bytes!("../../docs/icon.icns");

/// This tells each "App" what to do when it is clicked, whether it is a function, a message, or a display
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum AppCommand {
    Function(Function),
    Message(Message),
    Display,
}

/// The main app struct, that represents an "App"
///
/// This struct represents a command that rustcast can perform, providing the rustcast
/// the data needed to search for the app, to display the app in search results, and to actually
/// "run" the app.
#[derive(Debug, Clone)]
pub struct App {
    pub ranking: i32,
    pub open_command: AppCommand,
    pub desc: String,
    pub icons: AppIcon,
    pub display_name: String,
    pub search_name: String,
}

#[derive(Debug, Clone)]
pub enum AppIcon {
    IconFromFont(fn() -> Text<'static>),
    ImageHandle(iced::widget::image::Handle),
    None,
}

impl PartialEq for AppIcon {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IconFromFont(_), Self::IconFromFont(_)) => true,
            (Self::ImageHandle(l0), Self::ImageHandle(r0)) => l0 == r0,
            (AppIcon::None, AppIcon::None) => true,
            _ => false,
        }
    }
}

impl AppIcon {
    pub fn render(&self) -> Element<'static, Message> {
        match self {
            Self::IconFromFont(icon_fn) => container(
                icon_fn()
                    .center()
                    .size(20)
                    .height(Length::Fill)
                    .width(Length::Fill),
            )
            .style(|theme| container::Style {
                background: None,
                border: iced::Border {
                    color: theme.palette().text,
                    width: 0.,
                    radius: Radius::new(10),
                },
                ..Default::default()
            })
            .height(30)
            .width(30)
            .into(),
            Self::ImageHandle(handle) => Viewer::new(handle)
                .height(30)
                .width(30)
                .scale_step(0.)
                .into(),
            Self::None => space().into(),
        }
    }

    pub fn from_handle(handle: Option<Handle>) -> AppIcon {
        handle
            .map(|x| AppIcon::ImageHandle(x))
            .unwrap_or(AppIcon::None)
    }
}

impl PartialEq for App {
    fn eq(&self, other: &Self) -> bool {
        self.search_name == other.search_name
            && self.icons == other.icons
            && self.desc == other.desc
            && self.display_name == other.display_name
    }
}

impl App {
    pub fn new(name: String, icon: AppIcon, desc: String, command: AppCommand) -> Self {
        Self {
            ranking: 0,
            open_command: command,
            icons: icon,
            search_name: name.to_lowercase(),
            display_name: name,
            desc,
        }
    }
    /// A vec of all the emojis as App structs
    pub fn emoji_apps() -> Vec<App> {
        emojis::iter()
            .filter(|x| x.unicode_version() < emojis::UnicodeVersion::new(17, 13))
            .map(|x| App {
                ranking: 0,
                icons: AppIcon::None,
                display_name: x.to_string(),
                search_name: x.name().to_string(),
                open_command: AppCommand::Function(Function::CopyToClipboard(
                    ClipBoardContentType::Text(x.to_string()),
                )),
                desc: x.name().to_string(),
            })
            .collect()
    }
    /// This returns the basic apps that rustcast has, such as quiting rustcast and opening preferences
    pub fn basic_apps() -> Vec<App> {
        let app_version = option_env!("APP_VERSION").unwrap_or("Unknown Version");

        let ferris_handle =
            image::ImageReader::new(Cursor::new(include_bytes!("../../docs/ferris_rs.png")))
                .with_guessed_format()
                .unwrap()
                .decode()
                .ok()
                .map(|img| Handle::from_rgba(img.width(), img.height(), img.into_bytes()));

        vec![
            App {
                ranking: 0,
                open_command: AppCommand::Function(Function::OpenWebsite(
                    "https://ferris.rs".to_string(),
                )),
                icons: AppIcon::from_handle(ferris_handle),
                desc: "Easter Egg".to_string(),
                display_name: "Ferris Plushies".to_string(),
                search_name: "ferris.rs".to_string(),
            },
            App {
                ranking: 0,
                open_command: AppCommand::Function(Function::Quit),
                desc: RUSTCAST_DESC_NAME.to_string(),
                icons: AppIcon::IconFromFont(quit_icon),
                display_name: "Quit RustCast".to_string(),
                search_name: "quit".to_string(),
            },
            App {
                ranking: 0,
                open_command: AppCommand::Function(Function::QuitAllApps),
                desc: RUSTCAST_DESC_NAME.to_string(),
                icons: AppIcon::IconFromFont(quit_icon),
                display_name: "Quit All Apps".to_string(),
                search_name: "quit all apps".to_string(),
            },
            App {
                ranking: 0,
                open_command: AppCommand::Message(Message::OpenSettingsWindow),
                desc: RUSTCAST_DESC_NAME.to_string(),
                icons: AppIcon::IconFromFont(settings_icon),
                display_name: "Open RustCast Preferences".to_string(),
                search_name: "settings".to_string(),
            },
            App {
                ranking: 0,
                open_command: AppCommand::Message(Message::SwitchToPage(Page::EmojiSearch)),
                desc: RUSTCAST_DESC_NAME.to_string(),
                icons: AppIcon::IconFromFont(emoji_icon),
                display_name: "Search for an Emoji".to_string(),
                search_name: "emoji".to_string(),
            },
            App {
                ranking: 0,
                open_command: AppCommand::Message(Message::SwitchToPage(Page::ClipboardHistory)),
                desc: RUSTCAST_DESC_NAME.to_string(),
                icons: AppIcon::IconFromFont(clipboard_icon),
                display_name: "Clipboard History".to_string(),
                search_name: "clipboard".to_string(),
            },
            App {
                ranking: 0,
                open_command: AppCommand::Message(Message::SwitchToPage(Page::FileSearch)),
                desc: RUSTCAST_DESC_NAME.to_string(),
                icons: AppIcon::IconFromFont(filesearch_icon),
                display_name: "Search for a file".to_string(),
                search_name: "file search".to_string(),
            },
            App {
                ranking: 0,
                open_command: AppCommand::Message(Message::ReloadConfig),
                desc: RUSTCAST_DESC_NAME.to_string(),
                icons: AppIcon::IconFromFont(refresh_icon),
                display_name: "Reload RustCast".to_string(),
                search_name: "refresh".to_string(),
            },
            App {
                ranking: 0,
                open_command: AppCommand::Display,
                desc: RUSTCAST_DESC_NAME.to_string(),
                icons: AppIcon::IconFromFont(info_icon),
                display_name: format!("Current RustCast Version: {app_version}"),
                search_name: "version".to_string(),
            },
        ]
    }

    /// Window tiling actions (12 positions)
    pub fn window_apps() -> Vec<App> {
        use crate::platform::macos::window::TilePosition;

        let icons = icns_data_to_handle(ICNS_ICON.to_vec());

        let actions: &[(&str, TilePosition)] = &[
            ("Left Half", TilePosition::LeftHalf),
            ("Right Half", TilePosition::RightHalf),
            ("Top Half", TilePosition::TopHalf),
            ("Bottom Half", TilePosition::BottomHalf),
            ("Top Left Quarter", TilePosition::TopLeft),
            ("Top Right Quarter", TilePosition::TopRight),
            ("Bottom Left Quarter", TilePosition::BottomLeft),
            ("Bottom Right Quarter", TilePosition::BottomRight),
            ("Left Third", TilePosition::LeftThird),
            ("Center Third", TilePosition::CenterThird),
            ("Right Third", TilePosition::RightThird),
            ("Maximize", TilePosition::Maximize),
        ];

        actions
            .iter()
            .map(|(name, pos)| App {
                ranking: 0,
                open_command: AppCommand::Function(Function::TileWindow(pos.clone())),
                desc: "Window Tiling".to_string(),
                icons: AppIcon::from_handle(icons.clone()),
                display_name: name.to_string(),
                search_name: name.to_lowercase(),
            })
            .collect()
    }

    /// This renders the app into an iced element, allowing it to be displayed in the search results
    pub fn render(
        self,
        theme: crate::config::Theme,
        id_num: u32,
        focussed_id: u32,
        on_press: Option<Message>,
    ) -> iced::Element<'static, Message> {
        let focused = focussed_id == id_num;

        // Title + subtitle (Raycast style)
        let text_block = Text::new(self.display_name)
            .font(theme.font())
            .size(16)
            .wrapping(Wrapping::None)
            .color(theme.text_color(1.0));
        let subtitle_block = container(
            Text::new(self.desc)
                .font(theme.font())
                .size(13)
                .width(Length::Fill)
                .align_x(Alignment::End)
                .color(theme.text_color(0.55)),
        );

        let mut row = Row::new()
            .align_y(Alignment::Center)
            .width(Fill)
            .spacing(10)
            .height(40);

        if theme.show_icons {
            row = row.push(container(self.icons.render()).width(30).height(30));
        }
        row = row
            .push(container(text_block).width(Fill))
            .push(subtitle_block);

        let name = self.search_name.clone();
        let theme_clone = theme.clone();
        let is_favourite = self.ranking == -1;
        row = row.push(
            Button::new(Text::new("♥️").width(Length::Fill).align_x(Alignment::End))
                .on_press_with(move || Message::ToggleFavouriteApp(name.clone()))
                .width(20)
                .style(move |_, status| favourite_button_style(&theme_clone, status, is_favourite)),
        );

        let msg = on_press.or(match self.open_command.clone() {
            AppCommand::Function(func) => Some(Message::RunFunction(func)),
            AppCommand::Message(msg) => Some(msg),
            AppCommand::Display => None,
        });

        let theme_clone = theme.clone();

        let content = Button::new(row)
            .on_press_maybe(msg)
            .style(move |_, _| result_button_style(&theme_clone))
            .width(Fill)
            .padding(0)
            .height(40);

        container(content)
            .id(format!("result-{}", id_num))
            .style(move |_| result_row_container_style(&theme, focused))
            .padding(8)
            .width(Fill)
            .into()
    }
}
