use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use std::sync::Mutex;

use ars_core::{AriaAttr, AttrMap, Env, HtmlAttr, Machine as _, Service, StrongSend};
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

fn key_event(key: KeyboardKey) -> KeyboardEventData {
    KeyboardEventData {
        key,
        character: None,
        code: String::new(),
        repeat: false,
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
        is_composing: false,
    }
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
fn file_upload_on_props_changed_emits_set_props_for_capture_and_name() {
    let base = test_props();

    assert_eq!(
        Machine::on_props_changed(
            &base,
            &Props {
                capture: Some("user".into()),
                ..base.clone()
            },
        ),
        vec![Event::SetProps]
    );

    assert_eq!(
        Machine::on_props_changed(
            &base,
            &Props {
                name: Some("attachments".into()),
                ..base.clone()
            },
        ),
        vec![Event::SetProps]
    );
}

#[test]
fn file_upload_set_props_syncs_capture_and_name_into_context() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    assert_eq!(service.context().capture, None);
    assert_eq!(service.context().name, None);

    let result = service.set_props(Props {
        capture: Some("environment".into()),
        name: Some("avatar".into()),
        ..service.props().clone()
    });

    assert!(result.context_changed);
    assert_eq!(service.context().capture.as_deref(), Some("environment"));
    assert_eq!(service.context().name.as_deref(), Some("avatar"));

    let attrs = service.connect(&|_| {}).hidden_input_attrs();
    assert_eq!(attrs.get(&HtmlAttr::Capture), Some("environment"));
    assert_eq!(attrs.get(&HtmlAttr::Name), Some("avatar"));
}

#[test]
fn file_upload_disabling_during_drag_over_resets_to_idle() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));
    assert_eq!(service.state(), &State::DragOver);
    assert!(service.context().dragging);

    let result = service.set_props(Props {
        disabled: true,
        ..service.props().clone()
    });

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Idle);
    assert!(!service.context().dragging);
    assert_eq!(service.context().drag_counter, 0);
}

#[test]
fn file_upload_readonly_during_drag_over_resets_to_idle() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));
    assert_eq!(service.state(), &State::DragOver);

    let result = service.set_props(Props {
        readonly: true,
        ..service.props().clone()
    });

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Idle);
    assert!(!service.context().dragging);
    assert_eq!(service.context().drag_counter, 0);
}

#[test]
fn file_upload_set_files_reconciles_uploading_to_idle_when_no_uploading_files() {
    let mut service = uploading_service("file-1");
    assert_eq!(service.state(), &State::Uploading);

    let result = service.send(Event::SetFiles(Some(vec![item(
        "file-1",
        "doc.txt",
        Status::Complete,
    )])));

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Idle);
}

#[test]
fn file_upload_set_files_reconciles_idle_to_uploading_when_files_uploading() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    assert_eq!(service.state(), &State::Idle);

    let result = service.send(Event::SetFiles(Some(vec![item(
        "file-1",
        "a.png",
        Status::Uploading,
    )])));

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Uploading);
}

#[test]
fn file_upload_init_starts_uploading_when_seeded_with_uploading_file() {
    let service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Uploading)]),
        &Env::default(),
        &Messages::default(),
    );

    assert_eq!(service.state(), &State::Uploading);
    assert!(service.connect(&|_| {}).is_uploading());
}

#[test]
fn file_upload_init_stays_idle_without_uploading_files() {
    let service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    assert_eq!(service.state(), &State::Idle);
}

#[test]
fn file_upload_directory_accepts_multiple_files_without_multiple_prop() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").directory(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![
        raw_file("a.png", 10, "image/png"),
        raw_file("b.png", 10, "image/png"),
    ])));

    assert_eq!(
        service.context().files.get().len(),
        2,
        "directory upload accepts all files regardless of the multiple prop"
    );
    assert!(service.context().rejected_files.is_empty());
}

#[test]
fn file_upload_hidden_input_directory_marks_multiple() {
    let props = Props::new().id("upload").directory(true);
    let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());
    let attrs = api_with_props(props, ctx, State::Idle).hidden_input_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Multiple), Some("true"));
}

#[test]
fn file_upload_drag_enter_during_upload_keeps_uploading_state() {
    let mut service = uploading_service("file-1");
    assert_eq!(service.state(), &State::Uploading);

    let result = service.send(Event::DragEnter);

    assert_eq!(service.state(), &State::Uploading);
    assert!(service.context().dragging);
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceDropzoneActive)
    );
}

#[test]
fn file_upload_drag_leave_during_upload_clears_dragging_and_stays_uploading() {
    let mut service = uploading_service("file-1");
    drop(service.send(Event::DragEnter));
    assert!(service.context().dragging);

    let result = service.send(Event::DragLeave);

    assert_eq!(service.state(), &State::Uploading);
    assert!(!service.context().dragging);
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceDropzoneLeft)
    );
}

#[test]
fn file_upload_drop_during_upload_appends_files_and_stays_uploading() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .multiple(true)
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::StartUpload));
    drop(service.send(Event::DragEnter));
    let result = service.send(Event::Drop(vec![raw_file("b.png", 10, "image/png")]));

    assert_eq!(service.state(), &State::Uploading);
    assert_eq!(service.context().files.get().len(), 2);
    assert!(!service.context().dragging);
    assert!(result.pending_effects.iter().any(|effect| effect.name
        == Effect::AnnounceFilesAdded {
            count: 1,
            from_drop: true
        }));
}

#[test]
fn file_upload_retry_with_auto_upload_resumes_uploading() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .auto_upload(true)
            .default_files(vec![Item {
                status: Status::Failed("err".into()),
                error: Some("err".into()),
                ..item("file-1", "a.png", Status::Failed("err".into()))
            }]),
        &Env::default(),
        &Messages::default(),
    );

    assert_eq!(service.state(), &State::Idle);

    let result = service.send(Event::RetryFile {
        file_id: "file-1".into(),
    });

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Uploading);
    assert_eq!(service.context().files.get()[0].status, Status::Uploading);
}

#[test]
fn file_upload_generated_ids_do_not_reuse_after_removal() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "a.png",
        10,
        "image/png",
    )])));
    let first_id = service.context().files.get()[0].id.clone();

    drop(service.send(Event::RemoveFile {
        file_id: first_id.clone(),
    }));
    assert!(service.context().files.get().is_empty());

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "b.png",
        10,
        "image/png",
    )])));
    let second_id = service.context().files.get()[0].id.clone();

    assert_ne!(
        first_id, second_id,
        "generated file id must not be reused after the previous file was removed"
    );
}

#[test]
fn file_upload_generated_ids_avoid_collision_with_controlled_ids() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true),
        &Env::default(),
        &Messages::default(),
    );

    // Parent supplies a file in the "file-N" id namespace, then relinquishes
    // control so the queue becomes uncontrolled again.
    drop(service.send(Event::SetFiles(Some(vec![item(
        "file-1",
        "external.png",
        Status::Complete,
    )]))));
    drop(service.send(Event::SetFiles(None)));

    // User adds a file; the generated id must not collide with the supplied one
    // because the monotonic counter was advanced past it on SetFiles.
    drop(service.send(Event::FilesSelected(vec![raw_file(
        "new.png",
        10,
        "image/png",
    )])));

    let ids: Vec<String> = service
        .context()
        .files
        .get()
        .iter()
        .map(|file| file.id.clone())
        .collect();

    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();

    assert_eq!(
        ids.len(),
        unique.len(),
        "generated id collided with an externally supplied id: {ids:?}"
    );
}

#[test]
fn file_upload_progress_ignored_for_non_uploading_file() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true).default_files(vec![
            item("file-1", "a.png", Status::Pending),
            item("file-2", "b.png", Status::Pending),
        ]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::StartUpload));

    // Cancel file-1 while file-2 keeps uploading (machine stays in Uploading).
    drop(service.send(Event::CancelFile {
        file_id: "file-1".into(),
    }));
    assert_eq!(service.state(), &State::Uploading);

    // A late progress callback for the cancelled file must be ignored.
    drop(service.send(Event::UploadProgress {
        file_id: "file-1".into(),
        progress: 0.7,
    }));

    let cancelled = service
        .context()
        .files
        .get()
        .iter()
        .find(|file| file.id == "file-1")
        .expect("cancelled file remains in the queue")
        .clone();

    assert_eq!(cancelled.status, Status::Cancelled);
    assert!(
        cancelled.progress.abs() < f64::EPSILON,
        "progress for a non-uploading file must not advance"
    );

    // The still-uploading file continues to accept progress.
    drop(service.send(Event::UploadProgress {
        file_id: "file-2".into(),
        progress: 0.4,
    }));
    let uploading = service
        .context()
        .files
        .get()
        .iter()
        .find(|file| file.id == "file-2")
        .expect("uploading file remains in the queue")
        .clone();
    assert!((uploading.progress - 0.4).abs() < f64::EPSILON);
}

#[test]
fn file_upload_item_delete_trigger_disabled_when_inert() {
    let readonly = Props::new()
        .id("upload")
        .readonly(true)
        .default_files(vec![item("file-1", "a.png", Status::Pending)]);
    let disabled = Props::new()
        .id("upload")
        .disabled(true)
        .default_files(vec![item("file-1", "a.png", Status::Pending)]);

    for props in [readonly, disabled] {
        let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());
        let attrs = api_with_props(props, ctx, State::Idle).item_delete_trigger_attrs(0);

        assert_eq!(
            attrs.get(&HtmlAttr::Disabled),
            Some("true"),
            "delete trigger must be disabled when the component is inert"
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true"),
            "delete trigger must be aria-disabled when the component is inert"
        );
    }
}

#[test]
fn file_upload_item_delete_trigger_enabled_by_default() {
    let props =
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]);
    let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());
    let attrs = api_with_props(props, ctx, State::Idle).item_delete_trigger_attrs(0);

    assert_eq!(attrs.get(&HtmlAttr::Disabled), None);
    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), None);
}

#[test]
fn file_upload_item_keydown_delete_removes_focused_file() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true).default_files(vec![
            item("file-1", "a.png", Status::Pending),
            item("file-2", "b.png", Status::Pending),
        ]),
        &Env::default(),
        &Messages::default(),
    );

    service
        .connect(&send_event)
        .on_item_keydown(1, &key_event(KeyboardKey::Delete));

    assert_eq!(
        *events.lock().unwrap(),
        vec![Event::RemoveFile {
            file_id: "file-2".into()
        }]
    );
}

#[test]
fn file_upload_item_keydown_backspace_removes_focused_file() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true).default_files(vec![
            item("file-1", "a.png", Status::Pending),
            item("file-2", "b.png", Status::Pending),
        ]),
        &Env::default(),
        &Messages::default(),
    );

    service
        .connect(&send_event)
        .on_item_keydown(0, &key_event(KeyboardKey::Backspace));

    assert_eq!(
        *events.lock().unwrap(),
        vec![Event::RemoveFile {
            file_id: "file-1".into()
        }]
    );
}

#[test]
fn file_upload_item_keydown_other_key_is_ignored() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    service
        .connect(&send_event)
        .on_item_keydown(0, &key_event(KeyboardKey::Enter));

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn file_upload_accept_compound_extension_matches() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").accept(vec![".tar.gz".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "archive.tar.gz",
        100,
        "application/gzip",
    )])));

    assert_eq!(
        service.context().files.get().len(),
        1,
        "a compound extension like .tar.gz must match archive.tar.gz"
    );
}

#[test]
fn file_upload_accept_compound_extension_rejects_mismatch() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").accept(vec![".tar.gz".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "photo.png",
        100,
        "image/png",
    )])));

    assert!(service.context().files.get().is_empty());
}

#[test]
fn file_upload_drag_leave_to_idle_announces_dropzone_left() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));
    let result = service.send(Event::DragLeave);

    assert_eq!(service.state(), &State::Idle);
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceDropzoneLeft),
        "leaving the dropzone must emit AnnounceDropzoneLeft"
    );
}

#[test]
fn file_upload_nested_drag_leave_does_not_announce_dropzone_left() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));
    drop(service.send(Event::DragEnter));
    let result = service.send(Event::DragLeave);

    assert_eq!(service.state(), &State::DragOver);
    assert!(
        !result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceDropzoneLeft),
        "a nested drag-leave that stays in DragOver must not announce"
    );
}

#[test]
fn file_upload_files_selected_announces_files_added() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    let result = service.send(Event::FilesSelected(vec![raw_file(
        "a.png",
        10,
        "image/png",
    )]));

    assert!(
        result.pending_effects.iter().any(|effect| effect.name
            == Effect::AnnounceFilesAdded {
                count: 1,
                from_drop: false
            }),
        "selecting files must announce the added count (polite, not from drop)"
    );
}

#[test]
fn file_upload_drop_announces_files_added() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::DragEnter));
    let result = service.send(Event::Drop(vec![raw_file("a.png", 10, "image/png")]));

    assert!(
        result.pending_effects.iter().any(|effect| effect.name
            == Effect::AnnounceFilesAdded {
                count: 1,
                from_drop: true
            }),
        "dropping files must announce the added count (assertive, from drop)"
    );
}

#[test]
fn file_upload_rejected_files_announce_rejection_not_addition() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").max_file_size(50),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::FilesSelected(vec![raw_file(
        "big.png",
        100,
        "image/png",
    )]));

    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceFilesRejected { count: 1 }),
        "rejected files must announce the rejected count"
    );
    assert!(
        !result
            .pending_effects
            .iter()
            .any(|effect| matches!(effect.name, Effect::AnnounceFilesAdded { .. })),
        "no files were accepted, so no addition should be announced"
    );
}

#[test]
fn file_upload_upload_complete_announces_completion() {
    let mut service = uploading_service("file-1");

    let result = service.send(Event::UploadComplete {
        file_id: "file-1".into(),
    });

    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceUploadComplete),
        "completing an upload must announce completion"
    );
}

#[test]
fn file_upload_controlled_selection_surfaces_files_and_fires_callback() {
    let changed = Arc::new(Mutex::new(Vec::<Vec<Item>>::new()));
    let captured = Arc::clone(&changed);

    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .multiple(true)
            .files(vec![item("file-1", "a.png", Status::Complete)])
            .on_files_change(move |files: Vec<Item>| {
                captured.lock().unwrap().push(files);
            }),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::FilesSelected(vec![raw_file(
        "b.png",
        10,
        "image/png",
    )]));

    // Controlled mode still surfaces the new file via the API (optimistic sync)...
    assert_eq!(service.context().files.get().len(), 2);
    // ...and emits the FilesChanged effect so the parent can sync its prop.
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::FilesChanged)
    );

    // The effect's setup closure invokes on_files_change with the new queue.
    let send: StrongSend<Event> = Arc::new(|_| {});
    for effect in result.pending_effects {
        if effect.name == Effect::FilesChanged {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }
    }

    let recorded = changed.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].len(), 2);
    assert_eq!(recorded[0][1].name, "b.png");
}

#[test]
fn file_upload_remove_fires_files_change_effect() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::RemoveFile {
        file_id: "file-1".into(),
    });

    assert!(service.context().files.get().is_empty());
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::FilesChanged)
    );
}

#[test]
fn file_upload_clear_fires_files_change_effect() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::ClearFiles);

    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::FilesChanged)
    );
}

#[test]
fn file_upload_nested_drag_during_upload_tracks_counter_without_extra_announcements() {
    let mut service = uploading_service("file-1");

    drop(service.send(Event::DragEnter));
    let result = service.send(Event::DragEnter);
    assert_eq!(service.state(), &State::Uploading);
    assert_eq!(service.context().drag_counter, 2);
    assert!(service.context().dragging);
    // A nested enter must not re-announce the active dropzone.
    assert!(
        !result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceDropzoneActive)
    );

    let result = service.send(Event::DragLeave);
    assert_eq!(service.context().drag_counter, 1);
    assert!(service.context().dragging);
    assert!(
        !result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceDropzoneLeft)
    );
}

#[test]
fn file_upload_remove_nonexistent_file_is_noop() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::RemoveFile {
        file_id: "missing".into(),
    });

    assert_eq!(service.context().files.get().len(), 1);
    assert!(
        !result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::FilesChanged)
    );
}

#[test]
fn file_upload_clear_empty_queue_emits_no_files_change() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    let result = service.send(Event::ClearFiles);

    assert!(
        !result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::FilesChanged)
    );
}

#[test]
fn file_upload_files_change_effect_is_noop_without_callback() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::RemoveFile {
        file_id: "file-1".into(),
    });

    // Running the FilesChanged effect without an on_files_change callback is a
    // harmless no-op.
    let send: StrongSend<Event> = Arc::new(|_| {});
    for effect in result.pending_effects {
        if effect.name == Effect::FilesChanged {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }
    }

    assert!(service.context().files.get().is_empty());
}

#[test]
fn file_upload_complete_and_error_ignore_non_uploading_files_while_uploading() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true).default_files(vec![
            item("file-1", "a.png", Status::Pending),
            item("file-2", "b.png", Status::Pending),
        ]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::StartUpload));
    drop(service.send(Event::CancelFile {
        file_id: "file-1".into(),
    }));
    assert_eq!(service.state(), &State::Uploading);

    // A late completion for the cancelled file must be ignored (no announce).
    let complete = service.send(Event::UploadComplete {
        file_id: "file-1".into(),
    });
    assert!(
        !complete
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::AnnounceUploadComplete)
    );

    // A late error for the cancelled file must also be ignored.
    drop(service.send(Event::UploadError {
        file_id: "file-1".into(),
        error: "late".into(),
    }));

    let file_1 = service
        .context()
        .files
        .get()
        .iter()
        .find(|file| file.id == "file-1")
        .expect("file-1 present")
        .clone();
    assert_eq!(file_1.status, Status::Cancelled);
}

#[test]
fn file_upload_retry_non_failed_file_is_noop() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::RetryFile {
        file_id: "file-1".into(),
    }));

    assert_eq!(service.context().files.get()[0].status, Status::Pending);
}

#[test]
fn file_upload_single_mode_rejects_second_file_in_same_selection() {
    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

    drop(service.send(Event::FilesSelected(vec![
        raw_file("a.png", 10, "image/png"),
        raw_file("b.png", 10, "image/png"),
    ])));

    assert_eq!(service.context().files.get().len(), 1);
    assert_eq!(
        service.context().rejected_files[0].reason,
        RejectionReason::TooMany
    );
}

#[test]
fn file_upload_start_upload_without_pending_files_is_noop() {
    let mut idle = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());
    let result = idle.send(Event::StartUpload);
    assert!(!result.state_changed);
    assert_eq!(idle.state(), &State::Idle);

    let mut uploading = uploading_service("file-1");
    let result = uploading.send(Event::StartUpload);
    assert!(!result.state_changed);
    assert_eq!(uploading.state(), &State::Uploading);
}

#[test]
fn file_upload_on_props_changed_detects_each_output_prop() {
    let base = test_props();

    let cases = [
        Props {
            multiple: true,
            ..base.clone()
        },
        Props {
            accept: vec!["image/*".into()],
            ..base.clone()
        },
        Props {
            max_file_size: Some(1),
            ..base.clone()
        },
        Props {
            min_file_size: Some(1),
            ..base.clone()
        },
        Props {
            max_files: Some(1),
            ..base.clone()
        },
        Props {
            auto_upload: true,
            ..base.clone()
        },
        Props {
            directory: true,
            ..base.clone()
        },
    ];

    for new in cases {
        assert_eq!(
            Machine::on_props_changed(&base, &new),
            vec![Event::SetProps]
        );
    }
}

#[test]
fn file_upload_item_attrs_out_of_bounds_index_is_minimal() {
    let api = api_for_state(State::Idle);

    // No file at the index: the optional-file branches take their `None` path.
    assert_eq!(api.item_attrs(0).get(&HtmlAttr::Role), None);
    assert_eq!(
        api.item_delete_trigger_attrs(0)
            .get(&HtmlAttr::Aria(AriaAttr::Label)),
        None
    );
}

#[test]
fn file_upload_file_size_message_formats_each_unit() {
    let (_, ctx) = Machine::init(&test_props(), &Env::default(), &Messages::default());
    let format = |bytes| (ctx.messages.file_size)(bytes, &ctx.locale);

    assert!(format(500).contains("bytes"));
    assert!(format(2_048).contains("KB"));
    assert!(format(1_536).contains("KB")); // 1.5 KB (fractional)
    assert!(format(3_145_728).contains("MB"));
}

#[test]
fn file_upload_accept_normalizes_jpg_alias_to_jpeg() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").accept(vec!["image/jpeg".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "photo.jpg",
        10,
        "image/jpg",
    )])));

    assert_eq!(
        service.context().files.get().len(),
        1,
        "image/jpg must normalize to image/jpeg and match"
    );
}

#[test]
fn file_upload_accept_extension_pattern_matches_equal_mime() {
    // A dotted accept token also matches a file whose reported MIME equals the
    // token verbatim (the exact-match arm of the extension branch).
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").accept(vec![".png".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file("blob", 10, ".png")])));

    assert_eq!(service.context().files.get().len(), 1);
}

#[test]
fn file_upload_accept_dot_only_pattern_rejects() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").accept(vec![".".into()]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "a.png",
        10,
        "image/png",
    )])));

    assert!(service.context().files.get().is_empty());
}

#[test]
fn file_upload_trigger_readonly_sets_disabled() {
    let props = Props::new().id("upload").readonly(true);
    let (_, ctx) = Machine::init(&props, &Env::default(), &Messages::default());
    let attrs = api_with_props(props, ctx, State::Idle).trigger_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Disabled), Some("true"));
}

#[test]
fn file_upload_dropzone_attrs_reflect_dragging_flag() {
    let mut ctx = Machine::init(&test_props(), &Env::default(), &Messages::default()).1;
    ctx.dragging = true;

    let attrs = api_with_context(ctx, State::DragOver).dropzone_attrs();

    assert_eq!(attrs.get(&HtmlAttr::Data("ars-dragging")), Some("true"));
}

#[test]
fn file_upload_item_handlers_out_of_bounds_are_noop() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&events);
    let send_event = move |event| captured.lock().unwrap().push(event);

    let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());
    let api = service.connect(&send_event);
    api.on_item_delete_trigger_click(5);
    api.on_item_keydown(5, &key_event(KeyboardKey::Delete));

    assert!(events.lock().unwrap().is_empty());
}

#[test]
fn file_upload_is_max_files_reached_false_without_limit() {
    let service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );

    assert!(!service.connect(&|_| {}).is_max_files_reached());
}

#[test]
fn file_upload_cancel_non_uploading_file_while_uploading_is_ignored() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true).default_files(vec![
            item("file-1", "a.png", Status::Pending),
            item("file-2", "b.png", Status::Pending),
        ]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::StartUpload));
    drop(service.send(Event::UploadComplete {
        file_id: "file-1".into(),
    }));
    assert_eq!(service.state(), &State::Uploading);

    // Cancelling the already-complete file leaves it Complete.
    drop(service.send(Event::CancelFile {
        file_id: "file-1".into(),
    }));

    assert_eq!(
        service
            .context()
            .files
            .get()
            .iter()
            .find(|file| file.id == "file-1")
            .expect("file-1 present")
            .status,
        Status::Complete
    );
}

#[test]
fn file_upload_retry_ignores_non_matching_files() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").default_files(vec![
            Item {
                status: Status::Failed("e".into()),
                error: Some("e".into()),
                ..item("file-1", "a.png", Status::Failed("e".into()))
            },
            item("file-2", "b.png", Status::Pending),
        ]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::RetryFile {
        file_id: "file-1".into(),
    }));

    let files = service.context().files.get();
    assert_eq!(
        files.iter().find(|f| f.id == "file-1").unwrap().status,
        Status::Pending
    );
    assert_eq!(
        files.iter().find(|f| f.id == "file-2").unwrap().status,
        Status::Pending
    );
}

#[test]
fn file_upload_accepts_file_within_all_limits() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .max_files(5)
            .max_file_size(1_000)
            .min_file_size(10),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![raw_file(
        "a.png",
        100,
        "image/png",
    )])));

    assert_eq!(service.context().files.get().len(), 1);
    assert!(service.context().rejected_files.is_empty());
}

#[test]
fn file_upload_max_files_accepts_up_to_limit_then_rejects_in_same_selection() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true).max_files(2),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::FilesSelected(vec![
        raw_file("a.png", 10, "image/png"),
        raw_file("b.png", 10, "image/png"),
        raw_file("c.png", 10, "image/png"),
    ])));

    assert_eq!(service.context().files.get().len(), 2);
    assert_eq!(
        service.context().rejected_files[0].reason,
        RejectionReason::TooMany
    );
}

#[test]
fn file_upload_reconcile_enters_uploading_from_drag_over() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true).files(vec![item(
            "file-1",
            "a.png",
            Status::Pending,
        )]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::DragEnter));
    assert_eq!(service.state(), &State::DragOver);
    assert!(service.context().dragging);

    // Controlled queue begins uploading mid-drag: enter Uploading, keep dragging.
    let result = service.send(Event::SetFiles(Some(vec![item(
        "file-1",
        "a.png",
        Status::Uploading,
    )])));
    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Uploading);
    assert!(service.context().dragging);

    // The Uploading drag-leave path then clears the drag without ending the upload.
    drop(service.send(Event::DragLeave));
    assert_eq!(service.state(), &State::Uploading);
    assert!(!service.context().dragging);
}

#[test]
fn file_upload_disabling_during_upload_drag_clears_drag_but_keeps_uploading() {
    let mut service = uploading_service("file-1");

    drop(service.send(Event::DragEnter));
    assert!(service.context().dragging);

    let result = service.set_props(Props {
        disabled: true,
        ..service.props().clone()
    });

    assert_eq!(service.state(), &State::Uploading);
    assert!(!service.context().dragging);
    assert_eq!(service.context().drag_counter, 0);
    assert!(result.context_changed);
}

#[test]
fn file_upload_item_is_keyboard_focusable() {
    let api = api_with_files(vec![item("file-1", "a.png", Status::Pending)]);

    assert_eq!(api.item_attrs(0).get(&HtmlAttr::TabIndex), Some("0"));
}

#[test]
fn file_upload_open_file_picker_blocked_while_inert() {
    let disabled = Props::new().id("upload").disabled(true);
    let readonly = Props::new().id("upload").readonly(true);

    for props in [disabled, readonly] {
        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages::default());
        let result = service.send(Event::OpenFilePicker);

        assert!(
            result.pending_effects.is_empty(),
            "the file picker must not open while the component is disabled or read-only"
        );
    }
}

#[test]
fn file_upload_rejection_message_reflects_rejection_state() {
    // No rejections: None.
    assert!(api_for_state(State::Idle).rejection_message().is_none());

    // With rejections: a screen-reader summary.
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
    assert!(service.connect(&|_| {}).rejection_message().is_some());
}

#[test]
fn file_upload_controlled_auto_upload_callback_reports_uploading_queue() {
    let changed = Arc::new(Mutex::new(Vec::<Vec<Item>>::new()));
    let captured = Arc::clone(&changed);

    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .auto_upload(true)
            .files(Vec::new())
            .on_files_change(move |files: Vec<Item>| {
                captured.lock().unwrap().push(files);
            }),
        &Env::default(),
        &Messages::default(),
    );

    let result = service.send(Event::FilesSelected(vec![raw_file(
        "a.png",
        10,
        "image/png",
    )]));
    assert_eq!(service.state(), &State::Uploading);

    // The FilesChanged callback must report the post-StartUpload queue (Uploading),
    // not the intermediate Pending snapshot — otherwise the parent writes back a
    // stale queue that reverts the machine to Idle.
    let send: StrongSend<Event> = Arc::new(|_| {});
    for effect in result.pending_effects {
        if effect.name == Effect::FilesChanged {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }
    }

    let recorded = changed.lock().unwrap();
    let last = recorded.last().expect("on_files_change fired");
    assert_eq!(last[0].status, Status::Uploading);
}

#[test]
fn file_upload_upload_complete_while_dragging_returns_to_drag_over() {
    let mut service = uploading_service("file-1");

    drop(service.send(Event::DragEnter));
    assert!(service.context().dragging);

    // Last upload finishes while a drag is active: settle in DragOver (keeping
    // the drag flags) so the drag-leave path can clear them — not Idle, which
    // would swallow the DragLeave and strand `data-ars-dragging`.
    let result = service.send(Event::UploadComplete {
        file_id: "file-1".into(),
    });
    assert!(result.state_changed);
    assert_eq!(service.state(), &State::DragOver);
    assert!(service.context().dragging);

    drop(service.send(Event::DragLeave));
    assert_eq!(service.state(), &State::Idle);
    assert!(!service.context().dragging);
}

#[test]
fn file_upload_controlled_finish_while_dragging_returns_to_drag_over() {
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").multiple(true).files(vec![item(
            "file-1",
            "a.png",
            Status::Uploading,
        )]),
        &Env::default(),
        &Messages::default(),
    );
    assert_eq!(service.state(), &State::Uploading);

    drop(service.send(Event::DragEnter));
    assert!(service.context().dragging);

    // Controlled parent replaces the queue with no uploading items mid-drag:
    // settle in DragOver (drag flags preserved), not Idle (which would strand
    // the drag).
    let result = service.send(Event::SetFiles(Some(vec![item(
        "file-1",
        "a.png",
        Status::Complete,
    )])));
    assert!(result.state_changed);
    assert_eq!(service.state(), &State::DragOver);
    assert!(service.context().dragging);

    drop(service.send(Event::DragLeave));
    assert_eq!(service.state(), &State::Idle);
    assert!(!service.context().dragging);
}

#[test]
fn file_upload_retry_targets_specific_failed_file_after_a_mismatch() {
    // The first file does not match the retried id, exercising the id-comparison
    // branch in the will_retry scan before reaching the matching failed file.
    let mut service = Service::<Machine>::new(
        Props::new().id("upload").default_files(vec![
            item("file-1", "a.png", Status::Pending),
            Item {
                status: Status::Failed("e".into()),
                error: Some("e".into()),
                ..item("file-2", "b.png", Status::Failed("e".into()))
            },
        ]),
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::RetryFile {
        file_id: "file-2".into(),
    }));

    assert_eq!(
        service
            .context()
            .files
            .get()
            .iter()
            .find(|f| f.id == "file-2")
            .unwrap()
            .status,
        Status::Pending
    );
}

#[test]
fn file_upload_retry_non_failed_file_does_not_auto_start() {
    let mut service = Service::<Machine>::new(
        Props::new()
            .id("upload")
            .auto_upload(true)
            .default_files(vec![item("file-1", "a.png", Status::Pending)]),
        &Env::default(),
        &Messages::default(),
    );
    assert_eq!(service.state(), &State::Idle);

    // Retrying a file that is not failed must not chain StartUpload, which would
    // otherwise start unrelated pending files.
    drop(service.send(Event::RetryFile {
        file_id: "file-1".into(),
    }));

    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().files.get()[0].status, Status::Pending);
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
