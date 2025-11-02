// Кастомная тема оформления приложения

use super::messages::Message;
use super::Tab;
use iced::{widget::button, Element, Theme};

/// Создает темную тему для приложения
/// В Iced 0.12 кастомные темы создаются через расширение существующих
/// Glassmorphism эффекты будут реализованы через кастомные стили компонентов
pub fn dark_theme() -> Theme {
    Theme::Dark
}

/// Вспомогательная структура для создания кнопок вкладок
pub struct TabButton {
    name: &'static str,
    tab: Tab,
    is_active: bool,
}

impl TabButton {
    pub fn new(name: &'static str, tab: Tab, active_tab: Tab) -> Self {
        Self {
            name,
            tab,
            is_active: tab == active_tab,
        }
    }
}

impl<'a> From<TabButton> for Element<'a, Message> {
    fn from(tab_button: TabButton) -> Self {
        let style = if tab_button.is_active {
            // Стиль для активной вкладки
            iced::theme::Button::Primary
        } else {
            // Стиль для неактивной вкладки
            iced::theme::Button::Secondary
        };

        button(iced::widget::text(tab_button.name))
            .on_press(Message::TabSelected(tab_button.tab))
            .style(style)
            .into()
    }
}
