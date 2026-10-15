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
pub fn run() {
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let copies = Rc::new(Cell::new(0));
    let received = copies.clone();
    install(Rc::new(move |action| match action {
        Action::Dismiss => hide(),
        Action::Menu(MenuAction::Quit) => {
            NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).terminate(None)
        }
        Action::Menu(_) => notice("Ação nativa recebida"),
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
        let search = controller.ivars().search.get().unwrap();
        let window = search.window().unwrap();
        window.makeFirstResponder(Some(search));
        let event=NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(NSEventType::KeyDown,NSPoint::new(0.,0.),NSEventModifierFlags::empty(),0.,window.windowNumber(),None,&string("servico"),&string("servico"),false,0).unwrap();
        NSApplication::sharedApplication(mtm).sendEvent(&event);
        let focus = search.currentEditor().is_some();
        let typed = search.stringValue().to_string() == "servico";
        controller.render(true);
        controller.select(0);
        controller.activate(0, false);
        let frame = window.frame();
        let anchor = button.window().unwrap().frame();
        let content = controller.view();
        let body = window.convertRectToScreen(content.convertRect_toView(content.bounds(), None));
        let anchored = body.origin.y + body.size.height <= anchor.origin.y + 1.
            && (frame.origin.x + frame.size.width / 2. - anchor.origin.x - anchor.size.width / 2.)
                .abs()
                < 220.;
        let menu = controller.ivars().more.get().unwrap().menu().unwrap();
        let close_menu = menu.clone();
        let observed = popover.clone();
        let menu_visible = Rc::new(Cell::new(false));
        let result = menu_visible.clone();
        later(0.2, move || {
            result.set(observed.isShown());
            close_menu.cancelTracking();
        });
        // SAFETY: Native control target is the retained controller on this thread.
        unsafe {
            controller.ivars().more.get().unwrap().performClick(None);
        }
        let accent = controller.ivars().controls.borrow().len() == 1;
        search.setStringValue(&string("outro"));
        controller.render(true);
        controller.select(6);
        window.makeFirstResponder(Some(search));
        search
            .currentEditor()
            .unwrap()
            .setSelectedRange(NSRange::new(1, 2));
        let scroll = controller
            .ivars()
            .scroll
            .get()
            .unwrap()
            .contentView()
            .bounds()
            .origin;
        hide();
        let masked = controller
            .ivars()
            .rows
            .borrow()
            .iter()
            .all(|row| row.code.is_none());
        controller
            .ivars()
            .hidden_at
            .set(Some(Instant::now() - Duration::from_millis(29_999)));
        show(&button, synthetic_rows.clone(), "synthetic");
        let resume_debug = serde_json::json!({"query":search.stringValue().to_string(),"selected":controller.ivars().selected.get(),"scrollBefore":[scroll.x,scroll.y],"scrollAfter":[controller.ivars().scroll.get().unwrap().contentView().bounds().origin.x,controller.ivars().scroll.get().unwrap().contentView().bounds().origin.y],"caret":[search.currentEditor().unwrap().selectedRange().location,search.currentEditor().unwrap().selectedRange().length]});
        let resumed = search.stringValue().to_string() == "outro"
            && controller.ivars().selected.get() == Some(6)
            && controller
                .ivars()
                .scroll
                .get()
                .unwrap()
                .contentView()
                .bounds()
                .origin
                == scroll
            && search.currentEditor().unwrap().selectedRange() == NSRange::new(1, 2);
        let command = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(NSEventType::KeyDown,NSPoint::new(0.,0.),NSEventModifierFlags::Command,0.,window.windowNumber(),None,&string("f"),&string("f"),false,3).unwrap();
        window.makeFirstResponder(Some(&**controller.ivars().more.get().unwrap()));
        let cmd_f =
            controller.view().performKeyEquivalent(&command) && search.currentEditor().is_some();
        hide();
        controller
            .ivars()
            .hidden_at
            .set(Some(Instant::now() - Duration::from_secs(30)));
        expire_hidden();
        let expired =
            search.stringValue().is_empty() && controller.ivars().rows.borrow().is_empty();
        show(&button, synthetic_rows.clone(), "synthetic");
        let output =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../.artifacts/native-quick-access");
        std::fs::create_dir_all(&output).unwrap();
        let report = serde_json::json!({"syntheticOnly":true,"nativeSearchFocused":focus,"keyboardInputReceived":typed,"accentInsensitiveResults":accent,"copyActionReceived":copies.get()==1,"menuOpened":controller.ivars().menu_opened.get(),"popoverStayedOpenForMenu":menu_visible.get(),"popoverStillOpen":popover.isShown(),"anchoredBelowIcon":anchored,"hiddenCodesCleared":masked,"resumeBefore30Seconds":resumed,"resumeDetails":resume_debug,"expiresAt30Seconds":expired,"commandFFocusesSearch":cmd_f,"panelFrame":[frame.origin.x,frame.origin.y,frame.size.width,frame.size.height],"anchorFrame":[anchor.origin.x,anchor.origin.y,anchor.size.width,anchor.size.height]});
        std::fs::write(
            output.join("native-probe.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        later(0.15, move || {
            let view = controller.view();
            view.displayIfNeeded();
            if let Some(bitmap) = view.bitmapImageRepForCachingDisplayInRect(view.bounds()) {
                view.cacheDisplayInRect_toBitmapImageRep(view.bounds(), &bitmap);
                // SAFETY: Empty image encoding options; only this synthetic view is captured.
                if let Some(data) = unsafe {
                    bitmap.representationUsingType_properties(
                        NSBitmapImageFileType::PNG,
                        &NSDictionary::new(),
                    )
                } {
                    std::fs::write(output.join("native-probe.png"), data.to_vec()).unwrap();
                }
            }
            println!("{report}");
            NSApplication::sharedApplication(mtm).terminate(None);
        });
    });
    app.run();
}
