use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Page size for paginated inline keyboards.
pub const KEYBOARD_PAGE_SIZE: usize = 6;

/// A single keyboard item.
#[derive(Debug, Clone)]
pub struct KeyboardItem {
    pub label: String,
    pub callback_data: String,
}

/// Build a paginated inline keyboard from items.
///
/// Returns `(keyboard_markup, total_pages)`.
pub fn paginate_keyboard(
    items: &[KeyboardItem],
    page: usize,
    filter_prefix: Option<&str>,
) -> (InlineKeyboardMarkup, usize) {
    let filtered: Vec<&KeyboardItem> = match filter_prefix {
        Some(prefix) => items.iter().filter(|i| i.callback_data.starts_with(prefix)).collect(),
        None => items.iter().collect(),
    };

    let total_pages = (filtered.len() + KEYBOARD_PAGE_SIZE - 1) / KEYBOARD_PAGE_SIZE;
    let page = page.min(total_pages.saturating_sub(1));
    let start = page * KEYBOARD_PAGE_SIZE;
    let end = (start + KEYBOARD_PAGE_SIZE).min(filtered.len());

    let mut rows: Vec<Vec<InlineKeyboardButton>> = filtered[start..end]
        .iter()
        .map(|item| {
            vec![InlineKeyboardButton::new(
                &item.label,
                teloxide::types::InlineKeyboardButtonKind::CallbackData(
                    item.callback_data.clone(),
                ),
            )]
        })
        .collect();

    // Pagination row
    if total_pages > 1 {
        let mut nav_row = Vec::new();
        if page > 0 {
            nav_row.push(InlineKeyboardButton::new(
                "◀️ Prev",
                teloxide::types::InlineKeyboardButtonKind::CallbackData(
                    format!("noop_page"),
                ),
            ));
        }
        nav_row.push(InlineKeyboardButton::new(
            format!("{}/{}", page + 1, total_pages),
            teloxide::types::InlineKeyboardButtonKind::CallbackData("noop_page".to_string()),
        ));
        if page + 1 < total_pages {
            nav_row.push(InlineKeyboardButton::new(
                "Next ▶️",
                teloxide::types::InlineKeyboardButtonKind::CallbackData(
                    format!("noop_page"),
                ),
            ));
        }
        rows.push(nav_row);
    }

    (InlineKeyboardMarkup::new(rows), total_pages)
}
