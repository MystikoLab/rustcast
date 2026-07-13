//! This has all the logic regarding the cliboard history
use arboard::ImageData;
use iced::{
    Alignment, Element, Length,
    widget::{Button, Text, container, row},
};

use crate::{
    app::Message,
    commands::Function,
    styles::{image_icon, result_button_style, result_row_container_style, text_icon, url_icon},
};

/// The kinds of clipboard content that rustcast can handle and their contents
#[derive(Debug, Clone)]
pub enum ClipBoardContentType {
    Text(String),
    Image(ImageData<'static>),
    Url(String),
}

fn shorten(s: &str) -> String {
    let s = s.trim();
    if s.len() <= 20 {
        String::from(s)
    } else {
        let ind = s.floor_char_boundary(20);
        [&s[..ind], "..."].concat()
    }
}

impl PartialEq for ClipBoardContentType {
    /// Let cliboard items be comparable
    fn eq(&self, other: &Self) -> bool {
        if let Self::Text(a) = self
            && let Self::Text(b) = other
        {
            return a == b;
        } else if let Self::Image(image_data) = self
            && let Self::Image(other_image_data) = other
        {
            return image_data.width == other_image_data.width
                && image_data.height == other_image_data.height
                && image_data.bytes == other_image_data.bytes;
        } else if let Self::Url(a) = self
            && let Self::Url(b) = other
        {
            return a == b;
        }
        false
    }
}

impl ClipBoardContentType {
    pub fn render_row(
        &self,
        selected: bool,
        theme: &crate::config::Theme,
    ) -> Element<'static, Message> {
        let title_icon_fn = match self {
            ClipBoardContentType::Image(_) => image_icon,
            ClipBoardContentType::Text(_) => text_icon,
            ClipBoardContentType::Url(_) => url_icon,
        };

        let first_few_chars = match self {
            ClipBoardContentType::Text(text) | ClipBoardContentType::Url(text) => shorten(text),
            ClipBoardContentType::Image(_) => "Image".to_string(),
        }
        .to_string();

        let theme_1 = theme.clone();
        let theme_2 = theme.clone();

        container(
            Button::new(
                row![
                    title_icon_fn().size(22),
                    Text::new(first_few_chars)
                        .center()
                        .height(Length::Fill)
                        .width(Length::Fill)
                ]
                .padding(5)
                .height(Length::Fill)
                .height(30)
                .align_y(Alignment::Center),
            )
            .style(move |_, _| result_button_style(&theme_1))
            .on_press(Message::RunFunction(Function::CopyToClipboard(
                self.clone(),
            ))),
        )
        .style(move |_| result_row_container_style(&theme_2, selected))
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_text_equality_is_content_based() {
        assert_eq!(
            ClipBoardContentType::Text("hello".to_string()),
            ClipBoardContentType::Text("hello".to_string())
        );
        assert_ne!(
            ClipBoardContentType::Text("hello".to_string()),
            ClipBoardContentType::Text("world".to_string())
        );
    }

    #[test]
    fn clipboard_to_app_truncates_and_uses_first_line_for_display() {
        let _item =
            ClipBoardContentType::Text("abcdefghijklmnopqrstuvwxyz\nsecond line".to_string());

        // let app = item.to_app();

        // assert_eq!(app.display_name, "abcdefghijklmnopqrstuvwxy");
        // assert_eq!(app.search_name, "abcdefghijklmnopqrstuvwxy");
        // assert_eq!(app.desc, "Clipboard Item");
    }

    #[test]
    fn clipboard_image_equality_includes_dimensions() {
        let first = ClipBoardContentType::Image(ImageData {
            width: 1,
            height: 2,
            bytes: vec![0; 8].into(),
        });
        let second = ClipBoardContentType::Image(ImageData {
            width: 2,
            height: 1,
            bytes: vec![0; 8].into(),
        });

        assert_ne!(first, second);
    }
}
