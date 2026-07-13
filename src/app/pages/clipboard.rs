//! The elements for the clipboard history page
use iced::{
    ContentFit,
    border::Radius,
    widget::{
        Scrollable,
        image::{Handle, Viewer},
        rule,
        scrollable::{Direction, Scrollbar},
        text::Wrapping,
        text_input,
    },
};

use crate::{
    app::{Editable, pages::prelude::*},
    clipboard::ClipBoardContentType,
    styles::{
        delete_button_style, open_button_style, open_icon, settings_text_input_item_style,
        trash_icon, with_alpha,
    },
};

/// The clipboard view
///
/// Takes:
/// - the clipboard content to render,
/// - the id of which element is focussed,
/// - and the [`Theme`]
///
/// Returns:
/// - the iced Element to render
pub fn clipboard_view(
    query: String,
    clipboard_content: &[ClipBoardContentType],
    focussed_id: u32,
    theme: Theme,
) -> Element<'static, Message> {
    let theme_clone = theme.clone();
    let theme_clone_2 = theme.clone();
    if clipboard_content.is_empty() {
        return container(
            Text::new("Copy something to use the clipboard history")
                .font(theme.font())
                .size(30)
                .center()
                .wrapping(Wrapping::WordOrGlyph),
        )
        .height(Length::Fill)
        .width(Length::Fill)
        .style(move |_| result_row_container_style(&theme_clone, false))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into();
    }

    let viewport_content: Element<'static, Message> =
        match clipboard_content.get(focussed_id as usize) {
            Some(content) => viewport_content(content, &theme),
            None => Text::new("").into(),
        };

    let row_render_theme = theme.clone();
    let query = query.clone();
    container(Row::from_iter([
        container(
            Scrollable::with_direction(
                Column::from_iter(
                    clipboard_content
                        .iter()
                        .filter(|x| match x {
                            ClipBoardContentType::Text(data) | ClipBoardContentType::Url(data) => {
                                data.to_lowercase().contains(&query)
                            }
                            ClipBoardContentType::Image(_) => query == "image",
                        } || query.trim().is_empty())
                        .enumerate()
                        .map(|(i, content)| {
                            content.render_row(i == focussed_id as usize, &row_render_theme)
                        }),
                )
                .width((WINDOW_WIDTH + 50.) / 3.),
                Direction::Vertical(Scrollbar::hidden()),
            )
            .id("results"),
        )
        .height(10000)
        .style(move |_| result_row_container_style(&theme_clone_2, false))
        .into(),
        rule::vertical(0.3)
            .style(|iced_theme: &iced::Theme| rule::Style {
                color: with_alpha(iced_theme.palette().text, 0.7),
                radius: Radius::new(0),
                fill_mode: rule::FillMode::Full,
                snap: true,
            })
            .into(),
        container(viewport_content)
            .height(10000)
            .padding(10)
            .style(move |_| result_row_container_style(&theme_clone, false))
            .width(((WINDOW_WIDTH + 50.) / 3.) * 2.)
            .into(),
    ]))
    .height(280)
    .into()
}

fn viewport_content(content: &ClipBoardContentType, theme: &Theme) -> Element<'static, Message> {
    let viewer: Element<'static, Message> = match content {
        ClipBoardContentType::Text(txt) | ClipBoardContentType::Url(txt) => {
            Scrollable::with_direction(
                container(
                    Text::new(txt.to_owned())
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .align_x(Alignment::Start)
                        .font(theme.font())
                        .size(16),
                )
                .width(Length::Fill)
                .height(Length::Fill),
                Direction::Both {
                    vertical: Scrollbar::hidden(),
                    horizontal: Scrollbar::hidden(),
                },
            )
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
        }

        ClipBoardContentType::Image(data) => {
            let bytes = data.to_owned_img().into_owned_bytes();
            container(
                Viewer::new(
                    Handle::from_rgba(data.width as u32, data.height as u32, bytes.to_vec())
                        .clone(),
                )
                .content_fit(ContentFit::ScaleDown)
                .scale_step(0.)
                .max_scale(1.)
                .min_scale(1.),
            )
            .padding(10)
            .style(|_| container::Style {
                border: iced::Border {
                    color: iced::Color::WHITE,
                    width: 1.,
                    radius: Radius::new(0.),
                },
                ..Default::default()
            })
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
        }
    };

    let theme_clone = theme.clone();
    let theme_clone_2 = theme.clone();
    let horizontal_line = rule::horizontal(0.3).style(|them: &iced::Theme| rule::Style {
        color: with_alpha(them.palette().text, 0.7),
        radius: Radius::new(0),
        fill_mode: rule::FillMode::Full,
        snap: true,
    });
    let open_url_option = match content {
        ClipBoardContentType::Url(url) => Button::new(
            open_icon()
                .width(Length::Fill)
                .height(Length::Fill)
                .center(),
        )
        .on_press(Message::RunFunction(
            crate::commands::Function::OpenWebsite(url.clone()),
        ))
        .height(30)
        .width(30)
        .style(open_button_style)
        .into(),
        _ => iced::widget::space().width(0).into(),
    };
    Column::from_iter([
        viewer,
        horizontal_line.into(),
        container(
            Row::from_iter([
                open_url_option,
                iced::widget::space().width(Length::Fill).into(),
                Button::new(
                    trash_icon()
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center(),
                )
                .width(30)
                .height(30)
                .on_press(Message::EditClipboardHistory(Editable::Delete(
                    content.to_owned(),
                )))
                .style(move |_, _| delete_button_style(&theme_clone))
                .into(),
                Button::new("Clear")
                    .on_press(Message::ClearClipboardHistory)
                    .style(move |_, _| delete_button_style(&theme_clone_2))
                    .into(),
            ])
            .spacing(10),
        )
        .width(Length::Fill)
        .align_x(Alignment::End)
        .padding(10)
        .into(),
    ])
    .into()
}

#[allow(unused)]
fn editable_text(text: &str, theme: &Theme) -> Element<'static, Message> {
    let text_string = text.to_string();
    let theme_clone = theme.clone();
    container(
        text_input("Edit clipboard history text", text)
            .on_input(move |input| {
                Message::EditClipboardHistory(Editable::Update {
                    old: ClipBoardContentType::Text(text_string.clone()),
                    new: ClipBoardContentType::Text(input),
                })
            })
            .align_x(Alignment::Start)
            .size(16)
            .width(Length::Fill)
            .style(move |_, _| settings_text_input_item_style(&theme_clone))
            .font(theme.font()),
    )
    .height(Length::Fill)
    .width(Length::Fill)
    .into()
}
