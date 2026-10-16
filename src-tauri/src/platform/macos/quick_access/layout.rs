use objc2_foundation::NSSize;

pub(super) const WIDTH: f64 = 380.;
pub(super) const ROW_HEIGHT: f64 = 60.;
pub(super) const ACTION_HEIGHT: f64 = 28.;
pub(super) const ACTION_COUNT: usize = 5;
const CHROME_HEIGHT: f64 = 192.;
const MAX_HEIGHT: f64 = 480.;

pub(super) struct Layout {
    pub size: NSSize,
    pub list_height: f64,
    pub list_y: f64,
    pub message_y: f64,
    pub more_y: f64,
    pub actions_height: f64,
}
pub(super) fn panel(count: usize, expanded: bool) -> Layout {
    let actions_height = if expanded {
        ACTION_COUNT as f64 * ACTION_HEIGHT
    } else {
        0.
    };
    let chrome = CHROME_HEIGHT + actions_height;
    let list_height = (count as f64 * ROW_HEIGHT).min(MAX_HEIGHT - chrome);
    Layout {
        size: NSSize::new(WIDTH, chrome + list_height),
        list_height,
        list_y: 108. + actions_height,
        message_y: 84. + actions_height,
        more_y: 46. + actions_height,
        actions_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_lists_fit_their_rows_and_both_states_keep_the_existing_cap() {
        for expanded in [false, true] {
            let empty = panel(0, expanded);
            let one = panel(1, expanded);
            assert_eq!(one.list_height, ROW_HEIGHT);
            assert_eq!(one.size.height, empty.size.height + ROW_HEIGHT);
            assert_eq!(panel(1000, expanded).size, NSSize::new(380., 480.));
        }
        assert_eq!(panel(0, false).size.height, 192.);
        assert_eq!(panel(1, false).size.height, 252.);
        assert_eq!(panel(1, true).size.height, 392.);
    }
    #[test]
    fn expanding_reserves_actions_and_a_fixed_settings_footer_before_scrolling() {
        let collapsed = panel(12, false);
        let expanded = panel(12, true);
        assert_eq!(collapsed.list_height, 288.);
        assert_eq!(expanded.list_height, 148.);
        assert_eq!(expanded.actions_height, 140.);
        assert_eq!(expanded.more_y - expanded.actions_height, collapsed.more_y);
        assert_eq!(expanded.list_y + expanded.list_height, 396.);
        assert_eq!(collapsed.list_y + collapsed.list_height, 396.);
        assert!(expanded.more_y + ACTION_HEIGHT < expanded.message_y);
        assert!(expanded.message_y + 18. < expanded.list_y);
    }
}
