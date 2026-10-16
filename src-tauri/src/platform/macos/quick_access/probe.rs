//! Self-contained native UI smoke test. It cannot access a credential store.
use super::*;
use block2::RcBlock;
use objc2_foundation::{NSDictionary, NSRunLoop, NSRunLoopCommonModes, NSTimer};
use std::{path::Path, ptr::NonNull};

fn later(seconds: f64, work: impl Fn() + 'static) {
    let block = RcBlock::new(move |_: NonNull<NSTimer>| work());
    // SAFETY: Timer and callback are created and fired only on the main run loop.
    unsafe {
        let timer = NSTimer::timerWithTimeInterval_repeats_block(seconds, false, &block);
        NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes);
    }
}
fn panel() -> (Retained<Controller>, Retained<NSPopover>) {
    PANEL.with(|p| {
        let p = p.borrow();
        let p = p.as_ref().unwrap();
        (p.controller.clone(), p.popover.clone())
    })
}
fn capture(controller: &Controller, name: &str) {
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../.artifacts/native-quick-access");
    std::fs::create_dir_all(&output).unwrap();
    let view = controller.view();
    view.displayIfNeeded();
    if let Some(bitmap) = view.bitmapImageRepForCachingDisplayInRect(view.bounds()) {
        view.cacheDisplayInRect_toBitmapImageRep(view.bounds(), &bitmap);
        // SAFETY: Only the owned synthetic view is encoded, with no options.
        if let Some(data) = unsafe {
            bitmap.representationUsingType_properties(
                NSBitmapImageFileType::PNG,
                &NSDictionary::new(),
            )
        } {
            std::fs::write(output.join(name), data.to_vec()).unwrap();
        }
    }
}
fn click(button: &NSButton) {
    // SAFETY: Only native controls owned by this synthetic harness are invoked.
    unsafe {
        button.performClick(None);
    }
}
fn is_focused(window: &NSWindow, button: &NSButton) -> bool {
    window
        .firstResponder()
        .is_some_and(|responder| responder.isEqual(Some(button)))
}
fn next_is(view: &NSView, next: &NSView) -> bool {
    // SAFETY: Both views and their key-view loop are retained by this controller.
    unsafe { view.nextKeyView() }.is_some_and(|view| view.isEqual(Some(next)))
}
pub fn run() {
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let copies = Rc::new(Cell::new(0));
    let received = copies.clone();
    let menu_actions = Rc::new(RefCell::new(Vec::new()));
    let received_menu_actions = menu_actions.clone();
    install(Rc::new(move |action| match action {
        Action::Dismiss => hide(),
        Action::Menu(action) => {
            received_menu_actions.borrow_mut().push(action);
            notice("Ação nativa recebida");
        }
        Action::Copy(_) => {
            received.set(received.get() + 1);
            notice("Cópia sintética recebida");
        }
        _ => {}
    }));
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(-1.);
    let button = item.button(mtm).unwrap();
    button.setTitle(&string("OTP"));
    let account = AccountSummary {
        id: "00000000-0000-0000-0000-000000000001".into(),
        issuer: "Serviço sintético".into(),
        account: "conta@example.invalid".into(),
        kind: OtpKind::Totp,
        algorithm: crate::domain::account::OtpAlgorithm::SHA1,
        digits: 6,
        period: 30,
        next_counter: None,
        pinned: false,
    };
    let mut synthetic_rows = vec![Row {
        account: account.clone(),
        code: Some("123456".into()),
        remaining: Some(18),
    }];
    for index in 2..=12 {
        synthetic_rows.push(Row {
            account: AccountSummary {
                id: format!("00000000-0000-0000-0000-{index:012}"),
                issuer: "Outro".into(),
                account: format!("conta{index}@example.invalid"),
                ..account.clone()
            },
            code: Some("654321".into()),
            remaining: Some(22),
        });
    }
    let anchor_button = button.clone();
    let initial_rows = synthetic_rows.clone();
    later(0.1, move || {
        #[allow(deprecated)]
        NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
        show(&anchor_button, initial_rows.clone(), "synthetic");
    });
    later(0.5, move || {
        let (controller, popover) = panel();
        let ivars = controller.ivars();
        let search = ivars.search.get().unwrap();
        let window = search.window().unwrap();
        let more = ivars.more.get().unwrap();
        let settings = ivars.settings.get().unwrap();
        let actions = ivars.actions.get().unwrap();
        let group = ivars.action_group.get().unwrap();
        let scroll_view = ivars.scroll.get().unwrap();
        let document = ivars.document.get().unwrap();
        let settings_frame = settings.frame();
        // SAFETY: The hierarchy is retained and inspected only on the main thread.
        let settings_outside = unsafe { settings.superview() }
            .is_some_and(|parent| parent.isEqual(Some(&*controller.view())))
            && !settings.isHidden()
            && settings.isEnabled();
        let initially_collapsed = !ivars.expanded.get() && group.isHidden();
        let capped = popover.contentSize().height == 480.
            && document.frame().size.height > scroll_view.contentView().bounds().size.height
            && scroll_view.hasVerticalScroller();
        window.makeFirstResponder(Some(search));
        let event=NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(NSEventType::KeyDown,NSPoint::new(0.,0.),NSEventModifierFlags::empty(),0.,window.windowNumber(),None,&string("servico"),&string("servico"),false,0).unwrap();
        NSApplication::sharedApplication(mtm).sendEvent(&event);
        let focus = search.currentEditor().is_some();
        let typed = search.stringValue().to_string() == "servico";
        controller.render(true);
        controller.select(0);
        controller.activate(0, false);
        let accent = ivars.controls.borrow().len() == 1;
        let compact = popover.contentSize().height == 252. && window.frame().size.height < 295.;
        capture(&controller, "native-probe-one.png");
        let frame = window.frame();
        let anchor = button.window().unwrap().frame();
        let content = controller.view();
        let body = window.convertRectToScreen(content.convertRect_toView(content.bounds(), None));
        let anchored = body.origin.y + body.size.height <= anchor.origin.y + 1.
            && (frame.origin.x + frame.size.width / 2. - anchor.origin.x - anchor.size.width / 2.)
                .abs()
                < 220.;
        click(more);
        let expanded_inline = ivars.expanded.get()
            && !group.isHidden()
            && popover.isShown()
            && popover.contentSize().height == 392.
            && window.frame().size.height < 435.
            && group
                .window()
                .is_some_and(|host| host.isEqual(Some(&*window)))
            && more.isAccessibilityExpanded();
        let settings_pinned = settings_outside
            && settings.frame() == settings_frame
            && group.frame().origin.y > settings_frame.origin.y + settings_frame.size.height
            && group.frame().origin.y + group.frame().size.height <= more.frame().origin.y;
        let expanded_key_loop =
            next_is(more, &actions[0]) && next_is(actions.last().unwrap(), settings);
        capture(&controller, "native-probe-one-expanded.png");
        window.makeFirstResponder(Some(&*actions[2]));
        click(more);
        let collapse_focus = !ivars.expanded.get()
            && group.isHidden()
            && is_focused(&window, more)
            && next_is(more, settings);
        search.setStringValue(&string("outro"));
        controller.render(true);
        controller.select(6);
        window.makeFirstResponder(Some(search));
        search
            .currentEditor()
            .unwrap()
            .setSelectedRange(NSRange::new(1, 2));
        click(more);
        let expanded_capped = popover.contentSize().height == 480.
            && scroll_view.contentView().bounds().size.height == 148.
            && scroll_view.hasVerticalScroller()
            && settings.frame() == settings_frame;
        let scroll = scroll_view.contentView().bounds().origin;
        hide();
        let masked = ivars.rows.borrow().iter().all(|row| row.code.is_none());
        ivars
            .hidden_at
            .set(Some(Instant::now() - Duration::from_secs(29)));
        show(&button, synthetic_rows.clone(), "synthetic");
        let resumed = search.stringValue().to_string() == "outro"
            && ivars.selected.get() == Some(6)
            && scroll_view.contentView().bounds().origin == scroll
            && search
                .currentEditor()
                .is_some_and(|editor| editor.selectedRange() == NSRange::new(1, 2))
            && ivars.expanded.get()
            && !group.isHidden();
        window.makeFirstResponder(Some(&*actions[2]));
        hide();
        show(&button, synthetic_rows.clone(), "synthetic");
        let action_focus_resumed = ivars.expanded.get() && is_focused(&window, &actions[2]);
        window.makeFirstResponder(Some(&**settings));
        hide();
        show(&button, synthetic_rows.clone(), "synthetic");
        let settings_focus_resumed = is_focused(&window, settings);
        let command = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(NSEventType::KeyDown,NSPoint::new(0.,0.),NSEventModifierFlags::Command,0.,window.windowNumber(),None,&string("f"),&string("f"),false,3).unwrap();
        window.makeFirstResponder(Some(&**more));
        let cmd_f =
            controller.view().performKeyEquivalent(&command) && search.currentEditor().is_some();
        hide();
        ivars
            .hidden_at
            .set(Some(Instant::now() - Duration::from_secs(30)));
        expire_hidden();
        let expired = search.stringValue().is_empty()
            && ivars.rows.borrow().is_empty()
            && !ivars.expanded.get()
            && group.isHidden();
        show(&button, vec![synthetic_rows[0].clone()], "one-account");
        let single_account = popover.contentSize().height == 252.
            && window.frame().size.height < 295.
            && search.isEnabled();
        search.setStringValue(&string("sem-correspondencia"));
        controller.render(true);
        let no_results_editable = search.isEnabled()
            && ivars.controls.borrow().is_empty()
            && popover.contentSize().height == 192.;
        click(more);
        show(&button, vec![], "first-use");
        let epoch_reset = !ivars.expanded.get() && group.isHidden();
        let empty_shown = popover.isShown()
            && ivars.rows.borrow().is_empty()
            && ivars.controls.borrow().is_empty();
        let empty_search_disabled = !search.isEnabled() && search.currentEditor().is_none();
        let empty_focused = is_focused(&window, more);
        capture(&controller, "native-probe-empty.png");
        click(settings);
        let settings_collapsed_dispatched = *menu_actions.borrow() == vec![MenuAction::Settings]
            && group.isHidden()
            && settings.frame() == settings_frame;
        click(more);
        let labels: Vec<_> = actions
            .iter()
            .map(|item| item.title().to_string())
            .collect();
        let labels_match = labels
            == [
                "Nova entrada",
                "Escanear QR da tela",
                "Importar",
                "Exportar",
                "Sair",
            ]
            && settings.title().to_string() == "Configurações";
        let mut first_actions_enabled = more.isEnabled() && !group.isHidden();
        for index in [0, 2, 1, 3, 4] {
            first_actions_enabled &= actions[index].isEnabled();
            click(&actions[index]);
        }
        let first_actions_dispatched = *menu_actions.borrow()
            == vec![
                MenuAction::Settings,
                MenuAction::NewAccount,
                MenuAction::Import,
                MenuAction::Scan,
                MenuAction::Export,
                MenuAction::Quit,
            ];
        capture(&controller, "native-probe-empty-expanded.png");
        let lock = ivars.lock.get().unwrap();
        let title = ivars.title.get().unwrap();
        let header_lock = lock.frame().origin.x
            >= title.frame().origin.x + title.frame().size.width
            && lock.frame().origin.y == title.frame().origin.y
            && lock.frame().origin.y > search.frame().origin.y + search.frame().size.height;
        click(lock);
        let lock_dispatched = menu_actions.borrow().last() == Some(&MenuAction::Lock);
        show(&button, synthetic_rows.clone(), "synthetic");
        capture(&controller, "native-probe.png");
        click(more);
        controller.select(11);
        let footer_after_scroll = settings.frame() == settings_frame
            && !settings.isHidden()
            && group.frame().origin.y > settings.frame().origin.y + settings.frame().size.height;
        let output =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../.artifacts/native-quick-access");
        std::fs::create_dir_all(&output).unwrap();
        let report = serde_json::json!({
            "syntheticOnly": true,
            "nativeSearchFocused": focus,
            "keyboardInputReceived": typed,
            "accentInsensitiveResults": accent,
            "copyActionReceived": copies.get() == 1,
            "anchoredBelowIcon": anchored,
            "initiallyCollapsed": initially_collapsed,
            "accordionExpandsInline": expanded_inline,
            "accordionCollapseRestoresFocus": collapse_focus,
            "expandedKeyLoop": expanded_key_loop,
            "settingsOutsideAndPinned": settings_pinned,
            "settingsRemainsAfterScrolling": footer_after_scroll,
            "settingsWorksWithAccordionClosed": settings_collapsed_dispatched,
            "hiddenCodesCleared": masked,
            "resumeBefore30Seconds": resumed,
            "actionFocusResumed": action_focus_resumed,
            "settingsFocusResumed": settings_focus_resumed,
            "expiresAt30Seconds": expired,
            "newEpochCollapsesAccordion": epoch_reset,
            "commandFFocusesSearch": cmd_f,
            "emptyVaultShown": empty_shown,
            "emptyVaultActionsEnabled": first_actions_enabled,
            "emptyVaultActionsDispatched": first_actions_dispatched,
            "compactFilteredList": compact,
            "singleAccountCompact": single_account,
            "longListCappedAndScrollable": capped,
            "expandedLongListCappedAndScrollable": expanded_capped,
            "emptySearchDisabled": empty_search_disabled,
            "emptyFocusOnMore": empty_focused,
            "noResultsSearchEditable": no_results_editable,
            "actionLabelsMatch": labels_match,
            "lockBesideTitle": header_lock,
            "headerLockDispatched": lock_dispatched,
            "popoverStillOpen": popover.isShown()
        });
        std::fs::write(
            output.join("native-probe.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        later(0.15, move || {
            capture(&controller, "native-probe-expanded.png");
            println!("{report}");
            NSApplication::sharedApplication(mtm).terminate(None);
        });
    });
    app.run();
}
