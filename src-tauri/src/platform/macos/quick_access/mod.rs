//! Native presentation only. AppKit objects never leave the main thread.
use crate::domain::account::{AccountSummary, OtpKind};
use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::*;
use objc2_foundation::{
    MainThreadMarker, NSNotification, NSObjectProtocol, NSPoint, NSRange, NSRect, NSRectEdge,
    NSSize, NSString,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Row {
    pub account: AccountSummary,
    pub code: Option<String>,
    pub remaining: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Lock,
    NewAccount,
    Scan,
    Import,
    Export,
    Settings,
    Quit,
}
pub enum Action {
    Hidden,
    Dismiss,
    Interact,
    Copy(String),
    Next(String),
    Menu(MenuAction),
}
type Callback = Rc<dyn Fn(Action)>;
#[derive(Clone)]
enum Focus {
    Search(NSRange),
    Row(String, u8),
    More,
}
struct RowControls {
    id: String,
    name: Retained<NSButton>,
    code: Retained<NSButton>,
    remaining: Retained<NSTextField>,
    next: Option<Retained<NSButton>>,
}
struct Ivars {
    callback: Callback,
    search: OnceCell<Retained<NSSearchField>>,
    more: OnceCell<Retained<NSPopUpButton>>,
    menu_opened: Cell<bool>,
    scroll: OnceCell<Retained<NSScrollView>>,
    document: OnceCell<Retained<NSView>>,
    message: OnceCell<Retained<NSTextField>>,
    rows: RefCell<Vec<Row>>,
    controls: RefCell<Vec<RowControls>>,
    selected: Cell<Option<usize>>,
    hidden_at: Cell<Option<Instant>>,
    epoch: RefCell<String>,
    focused: RefCell<Focus>,
    rendered_query: RefCell<String>,
}
define_class!(
    // SAFETY: NSView has no extra subclass invariants; AppKit owns the view tree.
    #[unsafe(super = NSView)] #[thread_kind = MainThreadOnly]
    struct FlippedView;
    impl FlippedView { #[unsafe(method(isFlipped))] fn flipped(&self) -> bool { true } }
);
define_class!(
    // SAFETY: Responder changes are confined to the main thread.
    #[unsafe(super = NSView)] #[thread_kind = MainThreadOnly]
    struct ContainerView;
    impl ContainerView {
        #[unsafe(method(performKeyEquivalent:))]
        fn equivalent(&self, event: &NSEvent) -> bool {
            if event.modifierFlags().contains(NSEventModifierFlags::Command)
                && event.charactersIgnoringModifiers().is_some_and(|text| text.to_string().eq_ignore_ascii_case("f")) {
                if let (Some(window), Some(search)) = (self.window(), self.viewWithTag(42)) {
                    window.makeFirstResponder(Some(&search));
                }
                true
            } else { unsafe { msg_send![super(self), performKeyEquivalent: event] } }
        }
    }
);
define_class!(
    // SAFETY: The retained popover owns the controller and its delegate targets.
    #[unsafe(super = NSViewController)] #[thread_kind = MainThreadOnly] #[ivars = Ivars]
    struct Controller;
    unsafe impl NSObjectProtocol for Controller {}
    unsafe impl NSPopoverDelegate for Controller {
        #[unsafe(method(popoverWillClose:))]
        fn closing(&self, _: &NSNotification) { self.save_focus(); }
        #[unsafe(method(popoverDidClose:))]
        fn closed(&self, _: &NSNotification) {
            if self.ivars().hidden_at.get().is_none() { self.ivars().hidden_at.set(Some(Instant::now())); }
            self.clear_codes();
            (self.ivars().callback)(Action::Hidden);
        }
    }
    unsafe impl NSMenuDelegate for Controller {
        #[unsafe(method(menuWillOpen:))]
        fn menu_open(&self, _: &NSMenu) { self.ivars().menu_opened.set(true); }
    }
    unsafe impl NSTextFieldDelegate for Controller {}
    unsafe impl NSSearchFieldDelegate for Controller {}
    unsafe impl NSControlTextEditingDelegate for Controller {
        #[unsafe(method(control:textView:doCommandBySelector:))]
        fn command(&self, _: &NSControl, _: &NSTextView, command: Sel) -> bool {
            if command == sel!(cancelOperation:) { (self.ivars().callback)(Action::Dismiss); return true.into(); }
            if command == sel!(moveDown:) || command == sel!(moveUp:) {
                let length = self.ivars().controls.borrow().len();
                if length > 0 {
                    let current = self.ivars().selected.get();
                    let index = if command == sel!(moveDown:) { current.map_or(0, |i| (i + 1).min(length - 1)) } else { current.unwrap_or(1).saturating_sub(1) };
                    self.select(index);
                }
                return true.into();
            }
            if command == sel!(insertNewline:) {
                if let Some(index) = self.ivars().selected.get() { self.activate(index, false); }
                return true.into();
            }
            if command == sel!(insertTab:) {
                if let Some(window) = self.ivars().search.get().unwrap().window() {
                    let controls = self.ivars().controls.borrow();
                    let next = self.ivars().selected.get().and_then(|index| controls.get(index)).and_then(|row| row.next.as_ref());
                    if let Some(next) = next { window.makeFirstResponder(Some(next)); }
                    else { window.makeFirstResponder(Some(&**self.ivars().more.get().unwrap())); }
                }
                return true.into();
            }
            false
        }
    }
    impl Controller {
        #[unsafe(method(searchChanged:))]
        fn search_changed(&self, _: &NSSearchField) {
            // Ending field editing can repeat the action without changing text.
            // That notification must not clear retained selection or renew idle time.
            if self.ivars().search.get().unwrap().stringValue().to_string() == *self.ivars().rendered_query.borrow() { return; }
            (self.ivars().callback)(Action::Interact); self.ivars().selected.set(None); self.render(true);
        }
        #[unsafe(method(copyRow:))]
        fn copy_row(&self, sender: &NSButton) { self.button_action(sender, false); }
        #[unsafe(method(nextRow:))]
        fn next_row(&self, sender: &NSButton) { self.button_action(sender, true); }
        #[unsafe(method(menuAction:))]
        fn menu_action(&self, sender: &NSMenuItem) {
            let action = match sender.tag() { 1 => MenuAction::Lock, 2 => MenuAction::NewAccount, 3 => MenuAction::Scan, 4 => MenuAction::Import, 5 => MenuAction::Export, 6 => MenuAction::Settings, 7 => MenuAction::Quit, _ => return };
            (self.ivars().callback)(Action::Menu(action));
        }
        #[unsafe(method(cancelOperation:))]
        fn cancel(&self, _: Option<&objc2::runtime::AnyObject>) { (self.ivars().callback)(Action::Dismiss); }
    }
);
fn rect(x: f64, y: f64, w: f64, h: f64) -> NSRect {
    NSRect::new(NSPoint::new(x, y), NSSize::new(w, h))
}
fn string(text: &str) -> Retained<NSString> {
    NSString::from_str(text)
}
fn label(mtm: MainThreadMarker, text: &str, frame: NSRect, size: f64) -> Retained<NSTextField> {
    let view = NSTextField::labelWithString(&string(text), mtm);
    view.setFrame(frame);
    view.setFont(Some(&NSFont::systemFontOfSize(size)));
    view
}
fn fold(text: &NSString) -> String {
    text.stringByFoldingWithOptions_locale(
        objc2_foundation::NSStringCompareOptions::CaseInsensitiveSearch
            | objc2_foundation::NSStringCompareOptions::DiacriticInsensitiveSearch,
        None,
    )
    .to_string()
}
fn row_title(row: &Row) -> String {
    if row.account.issuer.is_empty() {
        row.account.account.clone()
    } else {
        format!("{}\n{}", row.account.issuer, row.account.account)
    }
}
impl Controller {
    fn new(mtm: MainThreadMarker, callback: Callback) -> Retained<Self> {
        let allocated = Self::alloc(mtm).set_ivars(Ivars {
            callback,
            search: OnceCell::new(),
            more: OnceCell::new(),
            menu_opened: Cell::new(false),
            scroll: OnceCell::new(),
            document: OnceCell::new(),
            message: OnceCell::new(),
            rows: RefCell::new(vec![]),
            controls: RefCell::new(vec![]),
            selected: Cell::new(None),
            hidden_at: Cell::new(None),
            epoch: RefCell::new(String::new()),
            focused: RefCell::new(Focus::Search(NSRange::new(0, 0))),
            rendered_query: RefCell::new(String::new()),
        });
        // SAFETY: Standard Cocoa initializers; delegate targets outlive views.
        let this: Retained<Self> = unsafe {
            msg_send![super(allocated), initWithNibName: std::ptr::null::<NSString>(), bundle: std::ptr::null::<objc2_foundation::NSBundle>()]
        };
        let root: Retained<ContainerView> =
            unsafe { msg_send![ContainerView::alloc(mtm), initWithFrame: rect(0.,0.,380.,480.)] };
        let title = label(mtm, "TopOTP", rect(16., 444., 348., 24.), 16.);
        let search =
            NSSearchField::initWithFrame(NSSearchField::alloc(mtm), rect(14., 406., 352., 28.));
        search.setTag(42);
        search.setPlaceholderString(Some(&string("Buscar contas…")));
        search.setAccessibilityLabel(Some(&string("Buscar contas")));
        search.setMaximumRecents(0);
        search.setRecentsAutosaveName(None);
        search.setSendsWholeSearchString(false);
        search.setSendsSearchStringImmediately(true);
        unsafe {
            search.setDelegate(Some(ProtocolObject::from_ref(&*this)));
            search.setTarget(Some(&this));
            search.setAction(Some(sel!(searchChanged:)));
        }
        let scroll =
            NSScrollView::initWithFrame(NSScrollView::alloc(mtm), rect(6., 64., 368., 332.));
        scroll.setHasVerticalScroller(true);
        scroll.setDrawsBackground(false);
        let document: Retained<FlippedView> =
            unsafe { msg_send![FlippedView::alloc(mtm), initWithFrame: rect(0.,0.,350.,332.)] };
        scroll.setDocumentView(Some(&document));
        let message = label(mtm, "", rect(16., 40., 348., 18.), 11.);
        message.setTextColor(Some(&NSColor::secondaryLabelColor()));
        let more = NSPopUpButton::initWithFrame_pullsDown(
            NSPopUpButton::alloc(mtm),
            rect(12., 8., 356., 28.),
            true,
        );
        let menu = NSMenu::new(mtm);
        menu.setAutoenablesItems(false);
        menu.setDelegate(Some(ProtocolObject::from_ref(&*this)));
        for (tag, title) in [
            "Mais",
            "Bloquear",
            "Nova entrada…",
            "Escanear QR da tela…",
            "Importar…",
            "Exportar…",
            "Configurações…",
            "Sair",
        ]
        .iter()
        .enumerate()
        {
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    NSMenuItem::alloc(mtm),
                    &string(title),
                    Some(sel!(menuAction:)),
                    &string(""),
                )
            };
            item.setTag(tag as isize);
            unsafe {
                item.setTarget(Some(&this));
            }
            menu.addItem(&item);
        }
        more.setMenu(Some(&menu));
        more.setBordered(false);
        // SAFETY: The next key view belongs to this retained view hierarchy.
        unsafe {
            more.setNextKeyView(Some(&search));
        }
        root.addSubview(&title);
        root.addSubview(&search);
        root.addSubview(&scroll);
        root.addSubview(&message);
        root.addSubview(&more);
        this.setView(&root);
        this.ivars().search.set(search).unwrap();
        this.ivars().more.set(more).unwrap();
        this.ivars().scroll.set(scroll).unwrap();
        this.ivars()
            .document
            .set(Retained::into_super(document))
            .unwrap();
        this.ivars().message.set(message).unwrap();
        this
    }
    fn button_action(&self, sender: &NSButton, next: bool) {
        let Some(id) = sender.identifier().map(|id| id.to_string()) else {
            return;
        };
        if let Some(index) = self
            .ivars()
            .controls
            .borrow()
            .iter()
            .position(|row| row.id == id)
        {
            self.ivars().selected.set(Some(index));
        }
        (self.ivars().callback)(Action::Interact);
        (self.ivars().callback)(if next {
            Action::Next(id)
        } else {
            Action::Copy(id)
        });
    }
    fn activate(&self, index: usize, next: bool) {
        let id = self
            .ivars()
            .controls
            .borrow()
            .get(index)
            .map(|row| row.id.clone());
        if let Some(id) = id {
            self.select(index);
            (self.ivars().callback)(Action::Interact);
            (self.ivars().callback)(if next {
                Action::Next(id)
            } else {
                Action::Copy(id)
            });
        }
    }
    fn select(&self, index: usize) {
        self.ivars().selected.set(Some(index));
        for (i, row) in self.ivars().controls.borrow().iter().enumerate() {
            row.name.setState(if i == index {
                NSControlStateValueOn
            } else {
                NSControlStateValueOff
            });
        }
        self.ivars()
            .document
            .get()
            .unwrap()
            .scrollRectToVisible(rect(0., index as f64 * 60., 350., 60.));
    }
    fn save_focus(&self) {
        let search = self.ivars().search.get().unwrap();
        if let Some(editor) = search.currentEditor() {
            *self.ivars().focused.borrow_mut() = Focus::Search(editor.selectedRange());
            return;
        }
        if let Some(responder) = search.window().and_then(|window| window.firstResponder()) {
            for row in self.ivars().controls.borrow().iter() {
                let part = if responder.isEqual(Some(&row.name)) {
                    Some(0)
                } else if responder.isEqual(Some(&row.code)) {
                    Some(1)
                } else if row
                    .next
                    .as_ref()
                    .is_some_and(|next| responder.isEqual(Some(next)))
                {
                    Some(2)
                } else {
                    None
                };
                if let Some(part) = part {
                    *self.ivars().focused.borrow_mut() = Focus::Row(row.id.clone(), part);
                    return;
                }
            }
            if responder.isEqual(Some(self.ivars().more.get().unwrap())) {
                *self.ivars().focused.borrow_mut() = Focus::More;
            }
        }
    }
    fn restore_focus(&self, window: &NSWindow) {
        let search = self.ivars().search.get().unwrap();
        match self.ivars().focused.borrow().clone() {
            Focus::Search(range) => {
                window.makeFirstResponder(Some(search));
                if let Some(editor) = search.currentEditor() {
                    let length = search.stringValue().length();
                    let location = range.location.min(length);
                    editor.setSelectedRange(NSRange::new(
                        location,
                        range.length.min(length - location),
                    ));
                }
            }
            Focus::More => {
                window.makeFirstResponder(Some(&**self.ivars().more.get().unwrap()));
            }
            Focus::Row(id, part) => {
                let rows = self.ivars().controls.borrow();
                if let Some(row) = rows.iter().find(|row| row.id == id) {
                    let button = match part {
                        1 => &row.code,
                        2 => row.next.as_ref().unwrap_or(&row.name),
                        _ => &row.name,
                    };
                    window.makeFirstResponder(Some(button));
                } else {
                    window.makeFirstResponder(Some(search));
                }
            }
        }
    }
    fn clear_codes(&self) {
        for row in self.ivars().rows.borrow_mut().iter_mut() {
            row.code = None;
            row.remaining = None;
        }
        for row in self.ivars().controls.borrow().iter() {
            row.code.setTitle(&string("— — —"));
            row.remaining.setStringValue(&string(""));
        }
    }
    fn reset(&self) {
        self.ivars().rows.borrow_mut().clear();
        self.ivars()
            .search
            .get()
            .unwrap()
            .setStringValue(&string(""));
        self.ivars().selected.set(None);
        self.ivars().hidden_at.set(None);
        self.ivars().epoch.borrow_mut().clear();
        *self.ivars().focused.borrow_mut() = Focus::Search(NSRange::new(0, 0));
        self.render(true);
        self.ivars()
            .document
            .get()
            .unwrap()
            .scrollPoint(NSPoint::new(0., 0.));
    }
    fn render(&self, force: bool) {
        *self.ivars().rendered_query.borrow_mut() =
            self.ivars().search.get().unwrap().stringValue().to_string();
        let query = fold(&self.ivars().search.get().unwrap().stringValue());
        let rows = self.ivars().rows.borrow();
        let filtered: Vec<_> = rows
            .iter()
            .filter(|row| {
                fold(&string(&format!(
                    "{} {}",
                    row.account.issuer, row.account.account
                )))
                .contains(&query)
            })
            .collect();
        let same = !force
            && self
                .ivars()
                .controls
                .borrow()
                .iter()
                .map(|row| (&row.id, row.next.is_some()))
                .eq(filtered
                    .iter()
                    .map(|row| (&row.account.id, row.account.kind == OtpKind::Hotp)));
        let mtm = self.mtm();
        if !same {
            self.ivars().controls.borrow_mut().clear();
            let document = self.ivars().document.get().unwrap();
            for child in document.subviews() {
                child.removeFromSuperview();
            }
            document.setFrame(rect(0., 0., 350., (filtered.len() as f64 * 60.).max(332.)));
            for (index, row) in filtered.iter().enumerate() {
                let name = unsafe {
                    NSButton::buttonWithTitle_target_action(
                        &string(&row_title(row)),
                        Some(self),
                        Some(sel!(copyRow:)),
                        mtm,
                    )
                };
                name.setFrame(rect(6., index as f64 * 60., 210., 56.));
                name.setAlignment(NSTextAlignment::Left);
                name.setBezelStyle(NSBezelStyle::AccessoryBar);
                name.setBordered(false);
                name.setButtonType(NSButtonType::PushOnPushOff);
                name.setFont(Some(&NSFont::systemFontOfSize(13.)));
                name.setIdentifier(Some(&string(&row.account.id)));
                if let Some(cell) = name.cell() {
                    cell.setWraps(true);
                    cell.setUsesSingleLineMode(false);
                }
                let code = unsafe {
                    NSButton::buttonWithTitle_target_action(
                        &string(""),
                        Some(self),
                        Some(sel!(copyRow:)),
                        mtm,
                    )
                };
                code.setFrame(rect(218., index as f64 * 60. + 2., 126., 30.));
                code.setBordered(false);
                code.setIdentifier(Some(&string(&row.account.id)));
                code.setFont(Some(&NSFont::monospacedDigitSystemFontOfSize_weight(
                    17., 0.5,
                )));
                let remaining = label(
                    mtm,
                    "",
                    rect(228., index as f64 * 60. + 33., 112., 16.),
                    10.,
                );
                remaining.setAlignment(NSTextAlignment::Right);
                remaining.setTextColor(Some(&NSColor::secondaryLabelColor()));
                let next = if row.account.kind == OtpKind::Hotp {
                    let next = unsafe {
                        NSButton::buttonWithTitle_target_action(
                            &string("Gerar próximo"),
                            Some(self),
                            Some(sel!(nextRow:)),
                            mtm,
                        )
                    };
                    next.setFrame(rect(228., index as f64 * 60. + 31., 112., 22.));
                    next.setBordered(false);
                    next.setFont(Some(&NSFont::systemFontOfSize(10.)));
                    next.setIdentifier(Some(&string(&row.account.id)));
                    next.setAccessibilityLabel(Some(&string(&format!(
                        "Gerar próximo código de {} {}",
                        row.account.issuer, row.account.account
                    ))));
                    Some(next)
                } else {
                    None
                };
                document.addSubview(&name);
                document.addSubview(&code);
                document.addSubview(&remaining);
                if let Some(next) = &next {
                    document.addSubview(next);
                }
                self.ivars().controls.borrow_mut().push(RowControls {
                    id: row.account.id.clone(),
                    name,
                    code,
                    remaining,
                    next,
                });
            }
        }
        for (control, row) in self.ivars().controls.borrow().iter().zip(filtered.iter()) {
            control.name.setTitle(&string(&row_title(row)));
            control.name.setAccessibilityLabel(Some(&string(&format!(
                "Copiar código de {} {}",
                row.account.issuer, row.account.account
            ))));
            let code = row
                .code
                .as_deref()
                .map(|code| format!("{} {}", &code[..code.len() / 2], &code[code.len() / 2..]))
                .unwrap_or_else(|| "— — —".into());
            control.code.setTitle(&string(&code));
            control.code.setEnabled(row.code.is_some());
            control.remaining.setStringValue(&string(
                &row.remaining
                    .map(|seconds| format!("{seconds} s"))
                    .unwrap_or_default(),
            ));
        }
        self.ivars()
            .message
            .get()
            .unwrap()
            .setStringValue(&string(if rows.is_empty() {
                "Adicione uma conta pelo menu Mais."
            } else if filtered.is_empty() {
                "Nenhuma conta encontrada."
            } else {
                "Clique em um código para copiar."
            }));
    }
}
struct Panel {
    controller: Retained<Controller>,
    popover: Retained<NSPopover>,
}
thread_local! { static PANEL: RefCell<Option<Panel>> = const { RefCell::new(None) }; }
pub fn install(callback: Callback) {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let controller = Controller::new(mtm, callback);
    let popover = NSPopover::new(mtm);
    popover.setContentViewController(Some(&controller));
    popover.setContentSize(NSSize::new(380., 480.));
    popover.setBehavior(NSPopoverBehavior::Transient);
    popover.setAnimates(false);
    popover.setDelegate(Some(ProtocolObject::from_ref(&*controller)));
    PANEL.with(|slot| {
        *slot.borrow_mut() = Some(Panel {
            controller,
            popover,
        })
    });
}
pub fn show(anchor: &NSStatusBarButton, rows: Vec<Row>, epoch: &str) {
    PANEL.with(|slot| {
        if let Some(panel) = slot.borrow().as_ref() {
            let ivars = panel.controller.ivars();
            if *ivars.epoch.borrow() != epoch
                || ivars
                    .hidden_at
                    .get()
                    .is_some_and(|at| at.elapsed() >= Duration::from_secs(30))
            {
                panel.controller.reset();
            }
            *ivars.epoch.borrow_mut() = epoch.into();
            ivars.hidden_at.set(None);
            *ivars.rows.borrow_mut() = rows;
            panel.controller.render(false);
            panel.popover.showRelativeToRect_ofView_preferredEdge(
                anchor.bounds(),
                anchor,
                NSRectEdge::MinY,
            );
            if let Some(window) = ivars.search.get().unwrap().window() {
                window.makeKeyAndOrderFront(None);
                panel.controller.restore_focus(&window);
            }
        }
    });
}
pub fn update(rows: Vec<Row>, epoch: &str) {
    PANEL.with(|slot| {
        if let Some(panel) = slot.borrow().as_ref()
            && panel.popover.isShown()
            && *panel.controller.ivars().epoch.borrow() == epoch
        {
            *panel.controller.ivars().rows.borrow_mut() = rows;
            panel.controller.render(false);
        }
    });
}
pub fn hide() {
    let popover = PANEL.with(|slot| slot.borrow().as_ref().map(|panel| panel.popover.clone()));
    if let Some(popover) = popover {
        popover.close();
    }
}
pub fn reset() {
    hide();
    PANEL.with(|slot| {
        if let Some(panel) = slot.borrow().as_ref() {
            panel.controller.reset();
        }
    });
}
pub fn expire_hidden() {
    PANEL.with(|slot| {
        if let Some(panel) = slot.borrow().as_ref()
            && !panel.popover.isShown()
            && panel
                .controller
                .ivars()
                .hidden_at
                .get()
                .is_some_and(|at| at.elapsed() >= Duration::from_secs(30))
        {
            panel.controller.reset();
        }
    });
}
pub fn notice(text: &str) {
    PANEL.with(|slot| {
        if let Some(panel) = slot.borrow().as_ref() {
            panel
                .controller
                .ivars()
                .message
                .get()
                .unwrap()
                .setStringValue(&string(text));
        }
    });
}
pub fn error_message(text: &str) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&string("TopOTP"));
    alert.setInformativeText(&string(text));
    alert.addButtonWithTitle(&string("OK"));
    let _ = alert.runModal();
}
#[cfg(feature = "native-ui-probe")]
mod probe;
#[cfg(feature = "native-ui-probe")]
pub use probe::run as probe;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_folds_unicode_without_changing_labels() {
        assert_eq!(fold(&string("Serviço ÁGUA")), "servico agua");
        assert!(fold(&string("AÇÃO")).contains("acao"));
    }
}
