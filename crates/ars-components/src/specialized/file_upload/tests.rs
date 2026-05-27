use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use std::sync::Mutex;

use ars_core::{AriaAttr, AttrMap, Env, HtmlAttr, Machine as _, Service};
use ars_interactions::{KeyboardEventData, KeyboardKey};
use insta::assert_snapshot;

use super::{
    Effect, Event, Item, Machine, Messages, Progress, Props, RawFile, RejectionReason, State,
    Status,
};

fn test_props() -> Props {
    Props::new().id("upload")
}

fn raw_file(name: &str, size: u64, mime_type: &str) -> RawFile {
    RawFile {
        name: name.to_string(),
        size,
        mime_type: mime_type.to_string(),
    }
}

fn item(id: &str, name: &str, status: Status) -> Item {
    Item {
        id: id.to_string(),
        name: name.to_string(),
        size: 1_024,
        mime_type: "image/png".to_string(),
        status,
        progress: 0.0,
        error: None,
    }
}

fn snapshot_attrs(attrs: &AttrMap) -> String {
    format!("{attrs:#?}")
}

fn api_for_state(state: State) -> super::Api<'static> {
    let props = Box::leak(Box::new(test_props()));

    let messages = Messages::default();

    let (_, mut ctx) = Machine::init(props, &Env::default(), &messages);

    ctx.messages = messages;

    let ctx = Box::leak(Box::new(ctx));
    let state = Box::leak(Box::new(state));
    let send = Box::leak(Box::new(|_: Event| {}));

    super::Api {
        state,
        ctx,
        props,
        send,
    }
}

fn api_with_files(files: Vec<Item>) -> super::Api<'static> {
    let props = Box::leak(Box::new(
        Props::new().id("upload").default_files(files.clone()),
    ));

    let messages = Messages::default();

    let (_, mut ctx) = Machine::init(props, &Env::default(), &messages);

    ctx.files.set(files);
    ctx.messages = messages;

    let ctx = Box::leak(Box::new(ctx));
    let state = Box::leak(Box::new(State::Idle));
    let send = Box::leak(Box::new(|_: Event| {}));

    super::Api {
        state,
        ctx,
        props,
        send,
    }
}

fn uploading_service(file_id: &str) -> Service<Machine> {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item(file_id, "doc.txt", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::StartUpload));

    service
}

fn api_with_context(ctx: super::Context, state: State) -> super::Api<'static> {
    api_with_props(test_props(), ctx, state)
}

fn api_with_props(props: Props, ctx: super::Context, state: State) -> super::Api<'static> {
    let props = Box::leak(Box::new(props));
    let ctx = Box::leak(Box::new(ctx));
    let state = Box::leak(Box::new(state));
    let send = Box::leak(Box::new(|_: Event| {}));

    super::Api {
        state,
        ctx,
        props,
        send,
    }
}

#[test]
fn file_upload_progress_fraction_handles_zero_total_and_partial_bytes() {
    assert!(
        (Progress {
            file_index: 0,
            bytes_sent: 0,
            bytes_total: 0,
        }
        .fraction()
            - 0.0)
            .abs()
            < f64::EPSILON
    );

    assert!(
        (Progress {
            file_index: 0,
            bytes_sent: 50,
            bytes_total: 100,
        }
        .fraction()
            - 0.5)
            .abs()
            < f64::EPSILON
    );
}

#[test]
fn file_upload_props_builder_sets_controlled_and_capture_fields() {
    let files = vec![item("file-1", "a.png", Status::Pending)];

    let controlled = Props::new().id("upload").files(files.clone());

    assert_eq!(controlled.files, Some(files.clone()));

    let captured = Props::new().capture("user");

    assert_eq!(captured.capture.as_deref(), Some("user"));

    let uncontrolled = Props::new()
        .id("upload")
        .files(files.clone())
        .uncontrolled();

    assert_eq!(uncontrolled.files, None);
}

#[test]
fn file_upload_api_accessors_reflect_context_and_state() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .max_files(1)
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    let api = service.connect(&|_| {});

    assert!(!api.is_uploading());
    assert_eq!(api.files().len(), 1);
    assert!(api.is_max_files_reached());
    assert!(api.rejected_files().is_empty());

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "b.png",
        10,
        "image/png",
    )])));

    let api = service.connect(&|_| {});

    assert_eq!(api.rejected_files().len(), 1);
}

#[test]
fn file_upload_api_is_uploading_true_in_uploading_state() {
    let service = uploading_service("file-1");

    assert!(service.connect(&|_| {}).is_uploading());
}

#[test]
fn file_upload_focus_sets_focused_part() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::Focus { part: "dropzone" }));

    assert_eq!(service.context().focused_part, Some("dropzone"));
}

#[test]
fn file_upload_blur_clears_focused_part() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::Focus { part: "dropzone" }));
    drop(service.send(Event::Blur { part: "dropzone" }));

    assert_eq!(service.context().focused_part, None);
}

#[test]
fn file_upload_disabled_blur_clears_focused_part() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").disabled(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::Focus { part: "dropzone" }));
    drop(service.send(Event::Blur { part: "dropzone" }));

    assert_eq!(service.context().focused_part, None);
}

#[test]
fn file_upload_props_default_matches_spec() {
    let props = Props::default();

    assert!(!props.disabled);
    assert!(!props.readonly);
    assert!(!props.auto_upload);
    assert!(props.files.is_none());
    assert!(props.default_files.is_empty());
}

#[test]
fn file_upload_idle_to_drag_over_on_drag_enter() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    let result = service.send(Event::DragEnter);

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::DragOver);
    assert!(service.context().dragging);
    assert_eq!(service.context().drag_counter, 1);
    assert_eq!(
        result.pending_effects[0].name,
        Effect::AnnounceDropzoneActive
    );
}

#[test]
fn file_upload_nested_drag_enter_increments_counter() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));
    drop(service.send(Event::DragEnter));

    assert_eq!(service.state(), &State::DragOver);
    assert_eq!(service.context().drag_counter, 2);
}

#[test]
fn file_upload_drag_leave_at_zero_returns_idle() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));

    let result = service.send(Event::DragLeave);

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Idle);
    assert!(!service.context().dragging);
    assert_eq!(service.context().drag_counter, 0);
}

#[test]
fn file_upload_nested_drag_leave_stays_drag_over_until_counter_zero() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));
    drop(service.send(Event::DragEnter));
    drop(service.send(Event::DragLeave));

    assert_eq!(service.state(), &State::DragOver);
    assert_eq!(service.context().drag_counter, 1);

    drop(service.send(Event::DragLeave));

    assert_eq!(service.state(), &State::Idle);
}

#[test]
fn file_upload_drop_from_drag_over_adds_files_and_returns_idle() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));

    let result = service.send(Event::Drop(vec![raw_file("a.png", 100, "image/png")]));

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().files.get().len(), 1);
    assert_eq!(service.context().files.get()[0].name, "a.png");
}

#[test]
fn file_upload_files_selected_appends_validated_files() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    let result = service.send(Event::FilesSelected(vec![raw_file(
        "b.pdf",
        200,
        "application/pdf",
    )]));

    assert!(!result.state_changed);
    assert_eq!(service.context().files.get().len(), 1);
}

#[test]
fn file_upload_auto_upload_chains_start_upload() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").auto_upload(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "c.png",
        50,
        "image/png",
    )])));

    assert_eq!(service.state(), &State::Uploading);
    assert_eq!(service.context().files.get()[0].status, Status::Uploading);
}

#[test]
fn file_upload_start_upload_moves_pending_to_uploading() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "doc.txt", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::StartUpload);

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Uploading);
    assert_eq!(service.context().files.get()[0].status, Status::Uploading);
}

#[test]
fn file_upload_upload_progress_updates_fraction() {
    let mut service = uploading_service("file-1");

    drop(service.send(Event::UploadProgress {
        file_id: "file-1".into(),
        progress: 0.5,
    }));

    assert!((service.context().files.get()[0].progress - 0.5).abs() < f64::EPSILON);
}

#[test]
fn file_upload_upload_complete_returns_idle_when_done() {
    let mut service = uploading_service("file-1");

    let result = service.send(Event::UploadComplete {
        file_id: "file-1".into(),
    });

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().files.get()[0].status, Status::Complete);
}

#[test]
fn file_upload_upload_error_marks_failed() {
    let mut service = uploading_service("file-1");

    drop(service.send(Event::UploadError {
        file_id: "file-1".into(),
        error: "network".into(),
    }));

    assert_eq!(
        service.context().files.get()[0].status,
        Status::Failed("network".into())
    );
}

#[test]
fn file_upload_accept_extension_pattern_matches_by_filename() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").accept(vec![".png".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "photo.png",
        100,
        "application/octet-stream",
    )])));

    assert_eq!(service.context().files.get().len(), 1);
}

#[test]
fn file_upload_cancel_file_marks_cancelled_and_returns_idle() {
    let mut service = uploading_service("file-1");

    let result = service.send(Event::CancelFile {
        file_id: "file-1".into(),
    });

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().files.get()[0].status, Status::Cancelled);
}

#[test]
fn file_upload_api_cancel_file_dispatches_event() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let events_capture = Arc::clone(&events);
    let send = move |event: Event| {
        events_capture.lock().unwrap().push(event);
    };

    let service = uploading_service("file-1");

    service.connect(&send).cancel_file("file-1");

    let recorded = events.lock().unwrap();

    assert_eq!(
        recorded.as_slice(),
        &[Event::CancelFile {
            file_id: "file-1".into(),
        }]
    );
}

#[test]
fn file_upload_on_props_changed_syncs_controlled_files_and_context_props() {
    let base = test_props();

    assert!(Machine::on_props_changed(&base, &base).is_empty());

    assert_eq!(
        Machine::on_props_changed(
            &base,
            &Props {
                files: Some(vec![item("ext-1", "external.png", Status::Pending)]),
                ..base.clone()
            },
        ),
        vec![Event::SetFiles(Some(vec![item(
            "ext-1",
            "external.png",
            Status::Pending
        )]))]
    );

    assert_eq!(
        Machine::on_props_changed(
            &base,
            &Props {
                required: true,
                ..base.clone()
            },
        ),
        vec![Event::SetProps]
    );

    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.set_props(Props {
        files: Some(vec![item("file-2", "b.png", Status::Complete)]),
        required: true,
        ..service.props().clone()
    });

    assert!(result.context_changed);
    assert_eq!(service.context().files.get().len(), 1);
    assert_eq!(service.context().files.get()[0].id, "file-2");
    assert!(service.context().required);
}

#[test]
fn file_upload_accept_exact_mime_pattern_matches() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .accept(vec!["application/pdf".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "doc.pdf",
        100,
        "application/pdf",
    )])));

    assert_eq!(service.context().files.get().len(), 1);
}

#[test]
fn file_upload_set_files_event_replaces_controlled_queue() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::SetFiles(Some(vec![item(
        "file-2",
        "b.png",
        Status::Complete,
    )]))));

    assert_eq!(service.context().files.get()[0].id, "file-2");
}

#[test]
fn file_upload_accept_rejects_invalid_mime() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").accept(vec!["image/*".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "doc.pdf",
        100,
        "application/pdf",
    )])));

    assert!(service.context().files.get().is_empty());
    assert_eq!(
        service.context().rejected_files[0].reason,
        RejectionReason::InvalidType
    );
}

#[test]
fn file_upload_max_file_size_rejects_large_files() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").max_file_size(50),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "big.png",
        100,
        "image/png",
    )])));

    assert!(service.context().files.get().is_empty());
    assert_eq!(
        service.context().rejected_files[0].reason,
        RejectionReason::TooLarge
    );
}

#[test]
fn file_upload_min_file_size_rejects_small_files() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").min_file_size(100),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "tiny.png",
        10,
        "image/png",
    )])));

    assert_eq!(
        service.context().rejected_files[0].reason,
        RejectionReason::TooSmall
    );
}

#[test]
fn file_upload_max_files_rejects_excess() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .max_files(1)
            .default_files(vec![item("file-1", "existing.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "new.png",
        100,
        "image/png",
    )])));

    assert_eq!(service.context().files.get().len(), 1);
    assert_eq!(
        service.context().rejected_files[0].reason,
        RejectionReason::TooMany
    );
}

#[test]
fn file_upload_remove_file_filters_queue() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").default_files(vec![
            item("file-1", "a.png", Status::Pending),
            item("file-2", "b.png", Status::Pending),
        ]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::RemoveFile {
        file_id: "file-1".into(),
    }));

    assert_eq!(service.context().files.get().len(), 1);
    assert_eq!(service.context().files.get()[0].id, "file-2");
}

#[test]
fn file_upload_clear_files_resets_queue() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::ClearFiles));

    assert_eq!(service.state(), &State::Idle);
    assert!(service.context().files.get().is_empty());
}

#[test]
fn file_upload_retry_failed_resets_to_pending() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").default_files(vec![Item {
            status: Status::Failed("err".into()),
            error: Some("err".into()),
            ..item("file-1", "a.png", Status::Failed("err".into()))
        }]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::RetryFile {
        file_id: "file-1".into(),
    }));

    assert_eq!(service.context().files.get()[0].status, Status::Pending);
    assert_eq!(service.context().files.get()[0].error, None);
}

#[test]
fn file_upload_disabled_blocks_file_selection() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").disabled(true),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::FilesSelected(vec![raw_file(
        "a.png",
        100,
        "image/png",
    )]));

    assert!(!result.state_changed);
    assert!(service.context().files.get().is_empty());
}

#[test]
fn file_upload_readonly_allows_focus_only() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").readonly(true),
        &Env::default(),
        &Messages::default(),
    );

    assert!(
        service
            .send(Event::Focus { part: "dropzone" })
            .context_changed
    );

    let remove = service.send(Event::RemoveFile {
        file_id: "file-1".into(),
    });

    assert!(!remove.state_changed);
    assert!(!remove.context_changed);
}

#[test]
fn file_upload_disabled_accepts_set_props_to_clear_disabled() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").disabled(true),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.set_props(Props {
        disabled: false,
        ..service.props().clone()
    });

    assert!(result.context_changed);
    assert!(!service.context().disabled);

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "a.png",
        100,
        "image/png",
    )])));

    assert_eq!(service.context().files.get().len(), 1);
}

#[test]
fn file_upload_disabled_applies_upload_progress_while_uploading() {
    let mut service = uploading_service("file-1");

    drop(service.set_props(Props {
        disabled: true,
        ..service.props().clone()
    }));

    drop(service.send(Event::UploadProgress {
        file_id: "file-1".into(),
        progress: 0.25,
    }));

    assert!((service.context().files.get()[0].progress - 0.25).abs() < f64::EPSILON);
}

#[test]
fn file_upload_uploading_auto_upload_starts_new_pending_files() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .multiple(true)
            .auto_upload(true)
            .default_files(vec![item("file-1", "first.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::StartUpload));

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "second.png",
        100,
        "image/png",
    )])));

    let files = service.context().files.get();
    assert_eq!(files.len(), 2);
    assert_eq!(files[1].status, Status::Uploading);
}

#[test]
fn file_upload_non_multiple_rejects_second_file() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(false),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "first.png",
        100,
        "image/png",
    )])));

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "second.png",
        100,
        "image/png",
    )])));

    assert_eq!(service.context().files.get().len(), 1);
    assert_eq!(
        service.context().rejected_files[0].reason,
        RejectionReason::TooMany
    );
}

#[test]
fn file_upload_upload_progress_clamps_invalid_fraction() {
    let mut service = uploading_service("file-1");

    drop(service.send(Event::UploadProgress {
        file_id: "file-1".into(),
        progress: 2.0,
    }));
    assert!((service.context().files.get()[0].progress - 1.0).abs() < f64::EPSILON);

    drop(service.send(Event::UploadProgress {
        file_id: "file-1".into(),
        progress: f64::NAN,
    }));
    assert!((service.context().files.get()[0].progress - 0.0).abs() < f64::EPSILON);
}

#[test]
fn file_upload_complete_ignores_cancelled_file() {
    let mut service = uploading_service("file-1");

    drop(service.send(Event::CancelFile {
        file_id: "file-1".into(),
    }));

    drop(service.send(Event::UploadComplete {
        file_id: "file-1".into(),
    }));

    assert_eq!(service.context().files.get()[0].status, Status::Cancelled);
}

#[test]
fn file_upload_open_file_picker_emits_effect() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    let result = service.send(Event::OpenFilePicker);

    assert_eq!(result.pending_effects[0].name, Effect::OpenFilePicker);
}

#[test]
fn file_upload_api_open_file_picker_sends_event() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    service.connect(&send_event).open_file_picker();

    assert_eq!(*events.lock().unwrap(), vec![Event::OpenFilePicker]);
}

#[test]
fn file_upload_dropzone_click_opens_picker() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    service.connect(&send_event).on_dropzone_click();

    assert_eq!(*events.lock().unwrap(), vec![Event::OpenFilePicker]);
}

#[test]
fn file_upload_dropzone_keydown_enter_opens_picker() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    service
        .connect(&send_event)
        .on_dropzone_keydown(&KeyboardEventData {
            key: KeyboardKey::Enter,
            character: None,
            code: String::new(),
            repeat: false,
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            is_composing: false,
        });

    assert_eq!(*events.lock().unwrap(), vec![Event::OpenFilePicker]);
}

#[test]
fn file_upload_hidden_input_change_selects_files() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    service
        .connect(&send_event)
        .on_hidden_input_change(vec![raw_file("x.png", 1, "image/png")]);

    assert_eq!(
        *events.lock().unwrap(),
        vec![Event::FilesSelected(vec![raw_file(
            "x.png",
            1,
            "image/png"
        )])]
    );
}

#[test]
fn file_upload_item_delete_trigger_removes_by_index() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-9", "nine.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    service.connect(&send_event).on_item_delete_trigger_click(0);

    assert_eq!(
        *events.lock().unwrap(),
        vec![Event::RemoveFile {
            file_id: "file-9".into()
        }]
    );
}

#[test]
fn file_upload_dropzone_attrs_include_button_role_and_state() {
    let attrs = api_for_state(State::DragOver).dropzone_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
    assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
    assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("drag-over"));
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
        Some("upload-label")
    );
}

#[test]
fn file_upload_dropzone_disabled_sets_aria_disabled() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.disabled = true;

    let attrs = api_with_context(ctx, State::Idle).dropzone_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
}

#[test]
fn file_upload_dropzone_readonly_sets_aria_disabled() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.readonly = true;

    let attrs = api_with_context(ctx, State::Idle).dropzone_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
}

#[test]
fn file_upload_messages_include_drag_and_drop_announcement_defaults() {
    let messages = Messages::default();

    let locale = ars_core::Locale::parse("en-US").expect("locale");

    assert_eq!(
        (messages.drop_zone_left)(&locale),
        "Drop zone is no longer active"
    );
    assert_eq!((messages.files_added)(2, &locale), "2 files added");
    assert_eq!((messages.trigger_label)(&locale), "Choose files to upload");
}

#[test]
fn file_upload_root_dragging_sets_data_flag() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.dragging = true;

    let attrs = api_with_context(ctx, State::DragOver).root_attrs();

    assert!(attrs.contains(&HtmlAttr::Data("ars-dragging")));
}

#[test]
fn file_upload_state_display_matches_data_state_tokens() {
    assert_eq!(State::Idle.to_string(), "idle");
    assert_eq!(State::DragOver.to_string(), "drag-over");
    assert_eq!(State::Uploading.to_string(), "uploading");
}

#[test]
fn file_upload_root_idle_snapshot() {
    assert_snapshot!(snapshot_attrs(&api_for_state(State::Idle).root_attrs()));
}

#[test]
fn file_upload_root_dragging_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.dragging = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::DragOver).root_attrs()
    ));
}

#[test]
fn file_upload_root_disabled_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.disabled = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).root_attrs()
    ));
}

#[test]
fn file_upload_root_readonly_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.readonly = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).root_attrs()
    ));
}

#[test]
fn file_upload_label_snapshot() {
    assert_snapshot!(snapshot_attrs(&api_for_state(State::Idle).label_attrs()));
}

#[test]
fn file_upload_dropzone_idle_snapshot() {
    assert_snapshot!(snapshot_attrs(&api_for_state(State::Idle).dropzone_attrs()));
}

#[test]
fn file_upload_dropzone_drag_over_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_for_state(State::DragOver).dropzone_attrs()
    ));
}

#[test]
fn file_upload_dropzone_uploading_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_for_state(State::Uploading).dropzone_attrs()
    ));
}

#[test]
fn file_upload_dropzone_disabled_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.disabled = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).dropzone_attrs()
    ));
}

#[test]
fn file_upload_dropzone_readonly_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.readonly = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).dropzone_attrs()
    ));
}

#[test]
fn file_upload_trigger_enabled_snapshot() {
    assert_snapshot!(snapshot_attrs(&api_for_state(State::Idle).trigger_attrs()));
}

#[test]
fn file_upload_trigger_disabled_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.disabled = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).trigger_attrs()
    ));
}

#[test]
fn file_upload_hidden_input_default_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_for_state(State::Idle).hidden_input_attrs()
    ));
}

#[test]
fn file_upload_hidden_input_multiple_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.multiple = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).hidden_input_attrs()
    ));
}

#[test]
fn file_upload_hidden_input_accept_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.accept = vec!["image/*".into()];

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).hidden_input_attrs()
    ));
}

#[test]
fn file_upload_hidden_input_directory_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.directory = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).hidden_input_attrs()
    ));
}

#[test]
fn file_upload_hidden_input_name_snapshot() {
    let props = Props::new().id("upload").name("attachments");

    let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());

    assert_snapshot!(snapshot_attrs(
        &api_with_props(props, ctx, State::Idle).hidden_input_attrs()
    ));
}

#[test]
fn file_upload_hidden_input_capture_snapshot() {
    let props = Props::new().id("upload").capture("user");

    let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());

    assert_snapshot!(snapshot_attrs(
        &api_with_props(props, ctx, State::Idle).hidden_input_attrs()
    ));
}

#[test]
fn file_upload_hidden_input_readonly_is_disabled() {
    let props = Props::new().id("upload").readonly(true);
    let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());
    let attrs = api_with_props(props, ctx, State::Idle).hidden_input_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Disabled), Some("true"));
}

#[test]
fn file_upload_hidden_input_required_only_when_queue_empty() {
    let props = Props::new()
        .id("upload")
        .required(true)
        .default_files(vec![item("file-1", "a.png", Status::Pending)]);
    let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());
    let attrs = api_with_props(props, ctx, State::Idle).hidden_input_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Required), None);
}

#[test]
fn file_upload_progress_fraction_clamps_above_one() {
    let fraction = Progress {
        file_index: 0,
        bytes_sent: 200,
        bytes_total: 100,
    }
    .fraction();

    assert!((fraction - 1.0).abs() < f64::EPSILON);
}

#[test]
fn file_upload_accept_wildcard_is_case_insensitive() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").accept(vec!["Image/*".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "photo.png",
        100,
        "image/png",
    )])));

    assert_eq!(service.context().files.get().len(), 1);
}

#[test]
fn file_upload_hidden_input_required_snapshot() {
    let props = Props::new().id("upload").required(true);

    let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());

    assert_snapshot!(snapshot_attrs(
        &api_with_props(props, ctx, State::Idle).hidden_input_attrs()
    ));
}

#[test]
fn file_upload_hidden_input_disabled_snapshot() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;

    ctx.disabled = true;

    assert_snapshot!(snapshot_attrs(
        &api_with_context(ctx, State::Idle).hidden_input_attrs()
    ));
}

#[test]
fn file_upload_item_group_with_files_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_with_files(vec![item("file-1", "photo.png", Status::Pending)]).item_group_attrs()
    ));
}

#[test]
fn file_upload_item_pending_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_with_files(vec![item("file-1", "photo.png", Status::Pending)]).item_attrs(0)
    ));
}

#[test]
fn file_upload_item_uploading_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_with_files(vec![item("file-1", "photo.png", Status::Uploading)]).item_attrs(0)
    ));
}

#[test]
fn file_upload_item_complete_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_with_files(vec![item("file-1", "photo.png", Status::Complete)]).item_attrs(0)
    ));
}

#[test]
fn file_upload_item_error_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_with_files(vec![item(
            "file-1",
            "photo.png",
            Status::Failed("network".into())
        )])
        .item_attrs(0)
    ));
}

#[test]
fn file_upload_item_cancelled_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_with_files(vec![item("file-1", "photo.png", Status::Cancelled)]).item_attrs(0)
    ));
}

#[test]
fn file_upload_item_delete_trigger_snapshot() {
    assert_snapshot!(snapshot_attrs(
        &api_with_files(vec![item("file-1", "photo.png", Status::Pending)])
            .item_delete_trigger_attrs(0)
    ));
}
