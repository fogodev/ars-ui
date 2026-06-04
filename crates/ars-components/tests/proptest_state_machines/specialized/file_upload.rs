use ars_components::specialized::file_upload::{
    Event, Machine, Props, RawFile, RejectionReason, State, Status,
};
use ars_core::{ConnectApi, Env, HtmlAttr, Service};
use proptest::prelude::*;

fn arb_raw_file() -> impl Strategy<Value = RawFile> {
    (
        "[a-z]{1,8}\\.(png|jpg|txt|pdf)",
        0u64..=10_000,
        prop_oneof![
            Just("image/png".to_string()),
            Just("image/jpeg".to_string()),
            Just("text/plain".to_string()),
            Just("application/pdf".to_string()),
        ],
    )
        .prop_map(|(name, size, mime_type)| RawFile {
            name,
            size,
            mime_type,
        })
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        Just(Event::DragEnter),
        Just(Event::DragLeave),
        prop::collection::vec(arb_raw_file(), 0..4).prop_map(Event::FilesSelected),
        prop::collection::vec(arb_raw_file(), 0..4).prop_map(Event::Drop),
        Just(Event::StartUpload),
        ("file-[0-5]", -1.0f64..2.0)
            .prop_map(|(file_id, progress)| Event::UploadProgress { file_id, progress }),
        "file-[0-5]".prop_map(|file_id| Event::UploadComplete { file_id }),
        ("file-[0-5]", "[a-z ]{0,16}")
            .prop_map(|(file_id, error)| Event::UploadError { file_id, error }),
        "file-[0-5]".prop_map(|file_id| Event::RemoveFile { file_id }),
        Just(Event::ClearFiles),
        "file-[0-5]".prop_map(|file_id| Event::RetryFile { file_id }),
        "file-[0-5]".prop_map(|file_id| Event::CancelFile { file_id }),
        Just(Event::OpenFilePicker),
        Just(Event::Focus { part: "dropzone" }),
        Just(Event::Blur { part: "dropzone" }),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn file_upload_event_sequences_preserve_queue_and_attr_invariants(
        multiple in any::<bool>(),
        directory in any::<bool>(),
        auto_upload in any::<bool>(),
        max_files in proptest::option::of(1usize..=4),
        max_file_size in proptest::option::of(1u64..=10_000),
        min_file_size in proptest::option::of(0u64..=1_000),
        events in prop::collection::vec(arb_event(), 0..96),
    ) {
        let props = Props {
            id: "upload".into(),
            multiple,
            directory,
            auto_upload,
            max_files,
            max_file_size,
            min_file_size,
            accept: vec!["image/*".into(), ".txt".into()],
            name: Some("files".into()),
            ..Props::default()
        };
        let mut svc = Service::<Machine>::new(props, &Env::default(), &Default::default());

        for ev in events {
            let mut result = svc.send(ev);

            result.pending_effects.clear();
        }

        match svc.state() {
            State::Idle | State::DragOver | State::Uploading => {}
        }

        let any_uploading = svc
            .context()
            .files
            .get()
            .iter()
            .any(|file| file.status == Status::Uploading);

        if *svc.state() == State::Uploading {
            prop_assert!(any_uploading || svc.context().dragging);
        }

        for file in svc.context().files.get() {
            prop_assert!((0.0..=1.0).contains(&file.progress));

            match &file.status {
                Status::Pending | Status::Uploading | Status::Complete | Status::Cancelled => {}
                Status::Failed(error) => prop_assert_eq!(file.error.as_ref(), Some(error)),
            }
        }

        for rejection in &svc.context().rejected_files {
            match rejection.reason {
                RejectionReason::InvalidType
                | RejectionReason::TooMany
                | RejectionReason::TooLarge { .. }
                | RejectionReason::TooSmall { .. }
                | RejectionReason::CustomValidation(_) => {}
            }
        }

        let api = svc.connect(&|_| {});

        let hidden = api.hidden_input_attrs();

        if multiple || directory {
            prop_assert!(hidden.contains(&HtmlAttr::Multiple));
        }

        if directory {
            prop_assert!(hidden.contains(&HtmlAttr::WebkitDirectory));
        }

        let dropzone = api.dropzone_attrs();

        let expected_state = svc.state().to_string();

        prop_assert_eq!(dropzone.get(&HtmlAttr::Role), Some("button"));
        prop_assert_eq!(dropzone.get(&HtmlAttr::Data("ars-state")), Some(expected_state.as_str()));
        prop_assert_eq!(api.part_attrs(ars_components::specialized::file_upload::Part::Dropzone), dropzone);
        prop_assert_eq!(api.part_attrs(ars_components::specialized::file_upload::Part::HiddenInput), hidden);

        if let Some(first) = svc.context().files.get().first() {
            let item = api.item_attrs(0);

            prop_assert_eq!(item.get(&HtmlAttr::Data("ars-file-id")), Some(first.id.as_str()));
            prop_assert_eq!(item.get(&HtmlAttr::Role), Some("listitem"));

            let delete = api.item_delete_trigger_attrs(0);

            prop_assert_eq!(delete.get(&HtmlAttr::Type), Some("button"));
        }
    }
}
