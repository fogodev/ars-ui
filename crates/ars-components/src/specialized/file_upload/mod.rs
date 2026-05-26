//! File upload component state machine and connect API.

use alloc::{format, string::String, vec::Vec};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HasId, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

/// Represents a single file in the upload queue.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Unique identifier for this file item.
    pub id: String,

    /// Original file name.
    pub name: String,

    /// File size in bytes.
    pub size: u64,

    /// MIME type (e.g., `"image/png"`).
    pub mime_type: String,

    /// Current upload status.
    pub status: Status,

    /// Upload progress as a fraction in `[0.0, 1.0]`.
    pub progress: f64,

    /// Error message if status is [`Status::Failed`].
    pub error: Option<String>,
}

/// Upload status for a single file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// File is queued but not yet uploading.
    Pending,

    /// File is currently being uploaded.
    Uploading,

    /// Upload completed successfully.
    Complete,

    /// Upload failed with an error message.
    Failed(String),

    /// Upload was cancelled by the user.
    Cancelled,
}

/// Granular upload progress information for a single file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    /// Index of the file in the upload queue.
    pub file_index: usize,

    /// Number of bytes sent so far.
    pub bytes_sent: usize,

    /// Total number of bytes to send.
    pub bytes_total: usize,
}

impl Progress {
    /// Progress as a fraction in `[0.0, 1.0]`.
    #[must_use]
    pub fn fraction(&self) -> f64 {
        if self.bytes_total == 0 {
            0.0
        } else {
            self.bytes_sent as f64 / self.bytes_total as f64
        }
    }
}

/// Reasons a file can be rejected before upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RejectionReason {
    /// File type not in the accepted list.
    InvalidType,

    /// File exceeds the maximum size.
    TooLarge,

    /// Adding this file would exceed the maximum count.
    TooMany,

    /// File is smaller than the minimum size.
    TooSmall,

    /// Custom validation failed.
    CustomValidation(String),
}

/// A file that was rejected during selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    /// Original file name.
    pub name: String,

    /// File size in bytes.
    pub size: u64,

    /// MIME type.
    pub mime_type: String,

    /// Reason for rejection.
    pub reason: RejectionReason,
}

/// The states for the `FileUpload` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Default state, waiting for user interaction.
    Idle,

    /// A file is being dragged over the dropzone.
    DragOver,

    /// One or more files are actively uploading.
    Uploading,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "idle",
            Self::DragOver => "drag-over",
            Self::Uploading => "uploading",
        })
    }
}

/// The events for the `FileUpload` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Files were dragged into the dropzone boundary.
    DragEnter,

    /// Files are being dragged over the dropzone.
    DragOver,

    /// Files left the dropzone boundary.
    DragLeave,

    /// Files were dropped onto the dropzone.
    Drop(Vec<RawFile>),

    /// User selected files via the native file input.
    FilesSelected(Vec<RawFile>),

    /// Begin uploading all pending files.
    StartUpload,

    /// Upload progress for a specific file.
    UploadProgress {
        /// The id of the file.
        file_id: String,

        /// The progress of the file.
        progress: f64,
    },

    /// Upload completed for a specific file.
    UploadComplete {
        /// The id of the file.
        file_id: String,
    },

    /// Upload failed for a specific file.
    UploadError {
        /// The id of the file.
        file_id: String,

        /// The error message.
        error: String,
    },

    /// Remove a file from the list.
    RemoveFile {
        /// The id of the file.
        file_id: String,
    },

    /// Clear all files.
    ClearFiles,

    /// Retry a failed upload.
    RetryFile {
        /// The id of the file.
        file_id: String,
    },

    /// Cancel an in-progress upload.
    CancelFile {
        /// The id of the file.
        file_id: String,
    },

    /// Open the native file picker.
    OpenFilePicker,

    /// Focus entered a part.
    Focus {
        /// The part that was focused.
        part: &'static str,
    },

    /// Focus left a part.
    Blur {
        /// The part that was blurred.
        part: &'static str,
    },

    /// Synchronize the externally controlled file list prop.
    SetFiles(Option<Vec<Item>>),

    /// Synchronize output-affecting props stored in context.
    SetProps,
}

/// Raw file data from the browser File API, prior to validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawFile {
    /// The name of the file.
    pub name: String,

    /// The size of the file in bytes.
    pub size: u64,

    /// The MIME type of the file.
    pub mime_type: String,
}

/// The context for the `FileUpload` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Currently accepted files.
    pub files: Bindable<Vec<Item>>,

    /// Files that were rejected during the most recent selection.
    pub rejected_files: Vec<Rejection>,

    /// Whether a drag is currently over the dropzone.
    pub dragging: bool,

    /// Component disabled state.
    pub disabled: bool,

    /// Read-only state (shows files but prevents adding/removing).
    pub readonly: bool,

    /// Whether a file is required.
    pub required: bool,

    /// Whether multiple files can be selected.
    pub multiple: bool,

    /// Accepted MIME types (e.g., `["image/*", "application/pdf"]`).
    pub accept: Vec<String>,

    /// Maximum file size in bytes.
    pub max_file_size: Option<u64>,

    /// Minimum file size in bytes.
    pub min_file_size: Option<u64>,

    /// Maximum number of files.
    pub max_files: Option<usize>,

    /// Whether to auto-start upload on file selection.
    pub auto_upload: bool,

    /// Whether the dropzone is a directory upload.
    pub directory: bool,

    /// Drag-over nesting counter (for nested elements).
    pub drag_counter: u32,

    /// Focused part.
    pub focused_part: Option<&'static str>,

    /// Locale for internationalized messages.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance base id.
    pub id: String,

    /// The id of the dropzone.
    pub dropzone_id: String,

    /// The id of the input.
    pub input_id: String,

    /// The id of the label.
    pub label_id: String,

    /// The id of the file list.
    pub file_list_id: String,
}

/// The props for the `FileUpload` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Controlled file list.
    pub files: Option<Vec<Item>>,

    /// Default files for uncontrolled mode.
    pub default_files: Vec<Item>,

    /// Disabled state.
    pub disabled: bool,

    /// Read-only state (shows files but prevents adding/removing).
    pub readonly: bool,

    /// Whether a file is required.
    pub required: bool,

    /// Allow multiple files.
    pub multiple: bool,

    /// Camera selection for mobile (`"user"` front, `"environment"` rear).
    pub capture: Option<String>,

    /// Accepted MIME types.
    pub accept: Vec<String>,

    /// Maximum file size in bytes.
    pub max_file_size: Option<u64>,

    /// Minimum file size in bytes.
    pub min_file_size: Option<u64>,

    /// Maximum number of files.
    pub max_files: Option<usize>,

    /// Auto-start upload after selection.
    pub auto_upload: bool,

    /// Allow directory upload.
    pub directory: bool,

    /// Name for form submission.
    pub name: Option<String>,

    /// Component instance ID.
    pub id: String,
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets controlled [`files`](Self::files).
    #[must_use]
    pub fn files(mut self, files: Vec<Item>) -> Self {
        self.files = Some(files);
        self
    }

    /// Clears controlled [`files`](Self::files), switching to uncontrolled mode.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.files = None;
        self
    }

    /// Sets [`default_files`](Self::default_files).
    #[must_use]
    pub fn default_files(mut self, files: Vec<Item>) -> Self {
        self.default_files = files;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Sets [`multiple`](Self::multiple).
    #[must_use]
    pub const fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Sets [`capture`](Self::capture).
    #[must_use]
    pub fn capture(mut self, capture: impl Into<String>) -> Self {
        self.capture = Some(capture.into());
        self
    }

    /// Sets [`accept`](Self::accept).
    #[must_use]
    pub fn accept(mut self, accept: Vec<String>) -> Self {
        self.accept = accept;
        self
    }

    /// Sets [`max_file_size`](Self::max_file_size).
    #[must_use]
    pub const fn max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = Some(max_file_size);
        self
    }

    /// Clears [`max_file_size`](Self::max_file_size).
    #[must_use]
    pub const fn clear_max_file_size(mut self) -> Self {
        self.max_file_size = None;
        self
    }

    /// Sets [`min_file_size`](Self::min_file_size).
    #[must_use]
    pub const fn min_file_size(mut self, min_file_size: u64) -> Self {
        self.min_file_size = Some(min_file_size);
        self
    }

    /// Clears [`min_file_size`](Self::min_file_size).
    #[must_use]
    pub const fn clear_min_file_size(mut self) -> Self {
        self.min_file_size = None;
        self
    }

    /// Sets [`max_files`](Self::max_files).
    #[must_use]
    pub const fn max_files(mut self, max_files: usize) -> Self {
        self.max_files = Some(max_files);
        self
    }

    /// Clears [`max_files`](Self::max_files).
    #[must_use]
    pub const fn clear_max_files(mut self) -> Self {
        self.max_files = None;
        self
    }

    /// Sets [`auto_upload`](Self::auto_upload).
    #[must_use]
    pub const fn auto_upload(mut self, auto_upload: bool) -> Self {
        self.auto_upload = auto_upload;
        self
    }

    /// Sets [`directory`](Self::directory).
    #[must_use]
    pub const fn directory(mut self, directory: bool) -> Self {
        self.directory = directory;
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

type LocaleMessageFn = dyn Fn(&Locale) -> String + Send + Sync;
type RemoveLabelMessageFn = dyn Fn(&str, &Locale) -> String + Send + Sync;
type RejectionCountMessageFn = dyn Fn(usize, &Locale) -> String + Send + Sync;
type FileSizeMessageFn = dyn Fn(u64, &Locale) -> String + Send + Sync;

/// Messages for the `FileUpload` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the upload root.
    pub dropzone_label: MessageFn<LocaleMessageFn>,

    /// Announcement when the dropzone becomes active.
    pub dropzone_active: MessageFn<LocaleMessageFn>,

    /// Announcement when the dropzone is no longer active.
    pub drop_zone_left: MessageFn<LocaleMessageFn>,

    /// Announcement when files were added via drop or selection.
    pub files_added: MessageFn<RejectionCountMessageFn>,

    /// Label for the browse trigger button.
    pub trigger_label: MessageFn<LocaleMessageFn>,

    /// Label for the file list container.
    pub file_list_label: MessageFn<LocaleMessageFn>,

    /// Label for a remove-file button.
    pub remove_label: MessageFn<RemoveLabelMessageFn>,

    /// Summary message when files were rejected.
    pub rejection_message: MessageFn<RejectionCountMessageFn>,

    /// Human-readable file size text.
    pub file_size: MessageFn<FileSizeMessageFn>,

    /// Error text when a file is too large.
    pub too_large: MessageFn<FileSizeMessageFn>,

    /// Error text when a file type is invalid.
    pub wrong_type: MessageFn<LocaleMessageFn>,

    /// Error text when too many files were selected.
    pub too_many_files: MessageFn<LocaleMessageFn>,

    /// Error text when a file is too small.
    pub too_small: MessageFn<FileSizeMessageFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            dropzone_label: MessageFn::static_str("Drag and drop files here, or click to browse"),
            dropzone_active: MessageFn::static_str("Drop files to upload"),
            drop_zone_left: MessageFn::static_str("Drop zone is no longer active"),
            files_added: MessageFn::new(|count: usize, _locale: &Locale| {
                format!("{count} files added")
            }),
            trigger_label: MessageFn::static_str("Choose files to upload"),
            file_list_label: MessageFn::static_str("Uploaded files"),
            remove_label: MessageFn::new(|name: &str, _locale: &Locale| format!("Remove {name}")),
            rejection_message: MessageFn::new(|count: usize, _locale: &Locale| {
                format!("{count} files rejected")
            }),
            file_size: MessageFn::new(|bytes: u64, locale: &Locale| {
                format_file_size_with_locale(bytes, locale)
            }),
            too_large: MessageFn::new(|max: u64, locale: &Locale| {
                format!(
                    "File exceeds maximum size of {}",
                    format_file_size_with_locale(max, locale)
                )
            }),
            wrong_type: MessageFn::static_str("File type not accepted"),
            too_many_files: MessageFn::static_str("Too many files selected"),
            too_small: MessageFn::new(|min: u64, locale: &Locale| {
                format!(
                    "File below minimum size of {}",
                    format_file_size_with_locale(min, locale)
                )
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the file upload machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter announces that the dropzone is active.
    AnnounceDropzoneActive,

    /// Adapter opens the native file picker via the hidden input.
    OpenFilePicker,
}

/// The machine for the `FileUpload` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let files = if let Some(files) = &props.files {
            Bindable::controlled(files.clone())
        } else {
            Bindable::uncontrolled(props.default_files.clone())
        };

        let ids = ComponentIds::from_id(&props.id);

        (
            State::Idle,
            Context {
                files,
                rejected_files: Vec::new(),
                dragging: false,
                disabled: props.disabled,
                readonly: props.readonly,
                required: props.required,
                multiple: props.multiple,
                accept: props.accept.clone(),
                max_file_size: props.max_file_size,
                min_file_size: props.min_file_size,
                max_files: props.max_files,
                auto_upload: props.auto_upload,
                directory: props.directory,
                drag_counter: 0,
                focused_part: None,
                locale: env.locale.clone(),
                messages: messages.clone(),
                id: ids.id().to_string(),
                dropzone_id: ids.part("dropzone"),
                input_id: ids.part("input"),
                label_id: ids.part("label"),
                file_list_id: ids.part("file-list"),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled || ctx.readonly {
            return match event {
                Event::Focus { part } => {
                    let part = *part;
                    Some(TransitionPlan::context_only(
                        move |context: &mut Context| {
                            context.focused_part = Some(part);
                        },
                    ))
                }

                Event::Blur { .. } => {
                    Some(TransitionPlan::context_only(|context: &mut Context| {
                        context.focused_part = None;
                    }))
                }

                _ => None,
            };
        }

        match (state, event) {
            (State::Idle, Event::DragEnter) => Some(
                TransitionPlan::to(State::DragOver)
                    .apply(|context: &mut Context| {
                        context.dragging = true;
                        context.drag_counter = 1;
                    })
                    .with_effect(PendingEffect::named(Effect::AnnounceDropzoneActive)),
            ),

            (State::DragOver, Event::DragEnter) => {
                Some(TransitionPlan::context_only(|context: &mut Context| {
                    context.drag_counter += 1;
                }))
            }

            (State::DragOver, Event::DragLeave) => {
                let new_counter = ctx.drag_counter.saturating_sub(1);

                if new_counter == 0 {
                    Some(
                        TransitionPlan::to(State::Idle).apply(|context: &mut Context| {
                            context.drag_counter = 0;
                            context.dragging = false;
                        }),
                    )
                } else {
                    Some(TransitionPlan::context_only(
                        move |context: &mut Context| {
                            context.drag_counter = new_counter;
                        },
                    ))
                }
            }

            (State::DragOver, Event::Drop(raw_files)) => {
                let raw = raw_files.clone();
                Some(apply_selected_files_plan(
                    Some(State::Idle),
                    raw,
                    ctx.auto_upload,
                    |context: &mut Context| {
                        context.dragging = false;
                        context.drag_counter = 0;
                    },
                ))
            }

            (State::Idle, Event::FilesSelected(raw_files))
            | (State::Uploading, Event::FilesSelected(raw_files)) => {
                let raw = raw_files.clone();
                let auto_upload = ctx.auto_upload;
                Some(apply_selected_files_plan(None, raw, auto_upload, |_| {}))
            }

            (State::Idle, Event::StartUpload) => {
                if !has_pending_files(ctx, props) {
                    return None;
                }

                Some(
                    TransitionPlan::to(State::Uploading).apply(|context: &mut Context| {
                        mark_pending_as_uploading(context);
                    }),
                )
            }

            (State::Uploading, Event::UploadProgress { file_id, progress }) => {
                let file_id = file_id.clone();
                let progress = *progress;
                Some(TransitionPlan::context_only(
                    move |context: &mut Context| {
                        update_file_progress(context, &file_id, progress);
                    },
                ))
            }

            (State::Uploading, Event::UploadComplete { file_id }) => {
                let file_id = file_id.clone();
                let still_uploading = upload_complete_updates(ctx, &file_id);
                Some(upload_finish_plan(
                    still_uploading,
                    move |context: &mut Context| {
                        apply_upload_complete(context, &file_id);
                    },
                ))
            }

            (State::Uploading, Event::UploadError { file_id, error }) => {
                let file_id = file_id.clone();
                let error = error.clone();
                let still_uploading = upload_error_updates(ctx, &file_id);
                Some(upload_finish_plan(
                    still_uploading,
                    move |context: &mut Context| {
                        apply_upload_error(context, &file_id, &error);
                    },
                ))
            }

            (_, Event::RemoveFile { file_id }) => {
                let file_id = file_id.clone();
                let still_uploading = remove_file_updates(ctx, &file_id);
                Some(file_queue_plan(
                    *state,
                    still_uploading,
                    move |context: &mut Context| {
                        remove_file_by_id(context, &file_id);
                    },
                ))
            }

            (_, Event::ClearFiles) => Some(TransitionPlan::to(State::Idle).apply(
                |context: &mut Context| {
                    context.files.set(Vec::new());
                    context.rejected_files.clear();
                },
            )),

            (_, Event::RetryFile { file_id }) => {
                let file_id = file_id.clone();
                Some(TransitionPlan::context_only(
                    move |context: &mut Context| {
                        retry_file_by_id(context, &file_id);
                    },
                ))
            }

            (_, Event::CancelFile { file_id }) => {
                let file_id = file_id.clone();
                let still_uploading = cancel_file_updates(ctx, &file_id);
                Some(file_queue_plan(
                    *state,
                    still_uploading,
                    move |context: &mut Context| {
                        cancel_file_by_id(context, &file_id);
                    },
                ))
            }

            (_, Event::OpenFilePicker) => Some(
                TransitionPlan::new().with_effect(PendingEffect::named(Effect::OpenFilePicker)),
            ),

            (_, Event::Focus { part }) => {
                let part = *part;
                Some(TransitionPlan::context_only(
                    move |context: &mut Context| {
                        context.focused_part = Some(part);
                    },
                ))
            }

            (_, Event::Blur { .. }) => {
                Some(TransitionPlan::context_only(|context: &mut Context| {
                    context.focused_part = None;
                }))
            }

            (_, Event::SetFiles(files)) => {
                let files = files.clone();
                Some(TransitionPlan::context_only(
                    move |context: &mut Context| {
                        if let Some(files) = files {
                            context.files.set(files.clone());
                            context.files.sync_controlled(Some(files));
                        } else {
                            context.files.sync_controlled(None);
                        }
                    },
                ))
            }

            (_, Event::SetProps) => Some(TransitionPlan::context_only({
                let disabled = props.disabled;
                let readonly = props.readonly;
                let required = props.required;
                let multiple = props.multiple;
                let accept = props.accept.clone();
                let max_file_size = props.max_file_size;
                let min_file_size = props.min_file_size;
                let max_files = props.max_files;
                let auto_upload = props.auto_upload;
                let directory = props.directory;

                move |context: &mut Context| {
                    context.disabled = disabled;
                    context.readonly = readonly;
                    context.required = required;
                    context.multiple = multiple;
                    context.accept = accept;
                    context.max_file_size = max_file_size;
                    context.min_file_size = min_file_size;
                    context.max_files = max_files;
                    context.auto_upload = auto_upload;
                    context.directory = directory;
                }
            })),

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "file_upload::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.files != new.files {
            events.push(Event::SetFiles(new.files.clone()));
        }

        if old.disabled != new.disabled
            || old.readonly != new.readonly
            || old.required != new.required
            || old.multiple != new.multiple
            || old.accept != new.accept
            || old.max_file_size != new.max_file_size
            || old.min_file_size != new.min_file_size
            || old.max_files != new.max_files
            || old.auto_upload != new.auto_upload
            || old.directory != new.directory
        {
            events.push(Event::SetProps);
        }

        events
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

/// DOM parts of the `FileUpload` component.
#[derive(ComponentPart)]
#[scope = "file-upload"]
pub enum Part {
    /// Root wrapper element.
    Root,

    /// Label describing the upload area.
    Label,

    /// Drag-and-drop target area.
    Dropzone,

    /// Button that opens the native file picker.
    Trigger,

    /// Container for file items.
    ItemGroup,

    /// A single file item.
    Item {
        /// Index of the file in the list.
        index: usize,
    },

    /// File name display for an item.
    ItemName {
        /// Index of the file in the list.
        index: usize,
    },

    /// File size text for an item.
    ItemSizeText {
        /// Index of the file in the list.
        index: usize,
    },

    /// Remove button for an item.
    ItemDeleteTrigger {
        /// Index of the file in the list.
        index: usize,
    },

    /// Upload progress indicator for an item.
    ItemProgress {
        /// Index of the file in the list.
        index: usize,
    },

    /// Hidden native file input.
    HiddenInput,
}

/// API for the `FileUpload` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("file_upload::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the component is currently dragging.
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        self.ctx.dragging
    }

    /// Whether the component is currently uploading.
    #[must_use]
    pub const fn is_uploading(&self) -> bool {
        matches!(self.state, State::Uploading)
    }

    /// Returns the accepted files in the queue.
    #[must_use]
    pub fn files(&self) -> &[Item] {
        self.ctx.files.get()
    }

    /// Returns files rejected during the most recent selection.
    #[must_use]
    pub fn rejected_files(&self) -> &[Rejection] {
        &self.ctx.rejected_files
    }

    /// Whether the maximum number of files has been reached.
    #[must_use]
    pub fn is_max_files_reached(&self) -> bool {
        is_max_files_reached(self.ctx, self.props)
    }

    /// Human-readable rejection announcement for screen readers.
    #[must_use]
    pub fn rejection_message(&self) -> Option<String> {
        if self.ctx.rejected_files.is_empty() {
            None
        } else {
            Some((self.ctx.messages.rejection_message)(
                self.ctx.rejected_files.len(),
                &self.ctx.locale,
            ))
        }
    }

    /// Opens the native file picker.
    pub fn open_file_picker(&self) {
        (self.send)(Event::OpenFilePicker);
    }

    /// Starts uploading all pending files.
    pub fn start_upload(&self) {
        (self.send)(Event::StartUpload);
    }

    /// Clears all files from the queue.
    pub fn clear_files(&self) {
        (self.send)(Event::ClearFiles);
    }

    /// Removes a file by id.
    pub fn remove_file(&self, file_id: &str) {
        (self.send)(Event::RemoveFile {
            file_id: file_id.to_string(),
        });
    }

    /// Retries a failed upload for the given file id.
    pub fn retry_file(&self, file_id: &str) {
        (self.send)(Event::RetryFile {
            file_id: file_id.to_string(),
        });
    }

    /// Cancels an in-progress upload for the given file id.
    pub fn cancel_file(&self, file_id: &str) {
        (self.send)(Event::CancelFile {
            file_id: file_id.to_string(),
        });
    }

    /// Root element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.dropzone_label)(&self.ctx.locale),
            );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        attrs
    }

    /// Label element attributes.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, &self.ctx.label_id)
            .set(HtmlAttr::For, &self.ctx.input_id);

        attrs
    }

    /// Dropzone element attributes.
    #[must_use]
    pub fn dropzone_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Dropzone.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, &self.ctx.dropzone_id)
            .set(HtmlAttr::Role, "button")
            .set(HtmlAttr::TabIndex, "0")
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(HtmlAttr::Aria(AriaAttr::LabelledBy), &self.ctx.label_id);

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Trigger button attributes.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.trigger_label)(&self.ctx.locale),
            );

        if self.ctx.disabled {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// File list container attributes.
    #[must_use]
    pub fn item_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemGroup.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, &self.ctx.file_list_id)
            .set(HtmlAttr::Role, "list")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.file_list_label)(&self.ctx.locale),
            );

        attrs
    }

    /// File item attributes at the given index.
    #[must_use]
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { index }.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if let Some(file) = self.ctx.files.get().get(index) {
            attrs
                .set(HtmlAttr::Role, "listitem")
                .set(HtmlAttr::Data("ars-state"), item_status_token(&file.status))
                .set(HtmlAttr::Data("ars-file-id"), &file.id)
                .set(
                    HtmlAttr::Aria(AriaAttr::Description),
                    (self.ctx.messages.file_size)(file.size, &self.ctx.locale),
                );
        }

        attrs
    }

    /// File item name attributes at the given index.
    #[must_use]
    pub fn item_name_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemName { index }.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// File item size text attributes at the given index.
    #[must_use]
    pub fn item_size_text_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemSizeText { index }.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// File item delete trigger attributes at the given index.
    #[must_use]
    pub fn item_delete_trigger_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemDeleteTrigger { index }.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if let Some(file) = self.ctx.files.get().get(index) {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.remove_label)(&file.name, &self.ctx.locale),
            );
        }

        attrs
    }

    /// File item progress indicator attributes at the given index.
    #[must_use]
    pub fn item_progress_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemProgress { index }.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Hidden file input attributes.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, &self.ctx.input_id)
            .set(HtmlAttr::Type, "file")
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Class, "ars-sr-input");

        if self.ctx.multiple {
            attrs.set_bool(HtmlAttr::Multiple, true);
        }

        if !self.ctx.accept.is_empty() {
            attrs.set(HtmlAttr::Accept, self.ctx.accept.join(","));
        }

        if self.ctx.directory {
            attrs.set(HtmlAttr::WebkitDirectory, "");
        }

        if let Some(ref capture) = self.props.capture {
            attrs.set(HtmlAttr::Capture, capture);
        }

        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }

        if self.ctx.required {
            attrs.set_bool(HtmlAttr::Required, true);
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Returns a human-readable validation error string for the given rejection.
    #[must_use]
    pub fn validation_error_text(&self, rejection: &Rejection) -> String {
        match &rejection.reason {
            RejectionReason::TooLarge => {
                (self.ctx.messages.too_large)(self.ctx.max_file_size.unwrap_or(0), &self.ctx.locale)
            }

            RejectionReason::InvalidType => (self.ctx.messages.wrong_type)(&self.ctx.locale),

            RejectionReason::TooMany => (self.ctx.messages.too_many_files)(&self.ctx.locale),

            RejectionReason::TooSmall => {
                (self.ctx.messages.too_small)(self.ctx.min_file_size.unwrap_or(0), &self.ctx.locale)
            }

            RejectionReason::CustomValidation(message) => message.clone(),
        }
    }

    /// Dispatches drag-enter intent.
    pub fn on_dropzone_drag_enter(&self) {
        (self.send)(Event::DragEnter);
    }

    /// Dispatches drag-over intent.
    pub fn on_dropzone_drag_over(&self) {
        (self.send)(Event::DragOver);
    }

    /// Dispatches drag-leave intent.
    pub fn on_dropzone_drag_leave(&self) {
        (self.send)(Event::DragLeave);
    }

    /// Dispatches drop intent with raw file metadata.
    pub fn on_dropzone_drop(&self, files: Vec<RawFile>) {
        (self.send)(Event::Drop(files));
    }

    /// Dispatches click intent to open the file picker.
    pub fn on_dropzone_click(&self) {
        (self.send)(Event::OpenFilePicker);
    }

    /// Dispatches keyboard intent on the dropzone.
    pub fn on_dropzone_keydown(&self, data: &KeyboardEventData) {
        if matches!(data.key, KeyboardKey::Enter | KeyboardKey::Space) {
            (self.send)(Event::OpenFilePicker);
        }
    }

    /// Dispatches trigger click intent to open the file picker.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::OpenFilePicker);
    }

    /// Dispatches hidden-input change intent with selected files.
    pub fn on_hidden_input_change(&self, files: Vec<RawFile>) {
        (self.send)(Event::FilesSelected(files));
    }

    /// Dispatches remove intent for the file at `index`.
    pub fn on_item_delete_trigger_click(&self, index: usize) {
        if let Some(file) = self.ctx.files.get().get(index) {
            (self.send)(Event::RemoveFile {
                file_id: file.id.clone(),
            });
        }
    }

    const fn state_token(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::DragOver => "drag-over",
            State::Uploading => "uploading",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Dropzone => self.dropzone_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ItemGroup => self.item_group_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::ItemName { index } => self.item_name_attrs(index),
            Part::ItemSizeText { index } => self.item_size_text_attrs(index),
            Part::ItemDeleteTrigger { index } => self.item_delete_trigger_attrs(index),
            Part::ItemProgress { index } => self.item_progress_attrs(index),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

/// Whether the maximum number of files has been reached.
fn is_max_files_reached(ctx: &Context, _props: &Props) -> bool {
    if let Some(max) = ctx.max_files {
        ctx.files.get().len() >= max
    } else {
        false
    }
}

/// Whether there are any pending files.
fn has_pending_files(ctx: &Context, _props: &Props) -> bool {
    ctx.files
        .get()
        .iter()
        .any(|file| file.status == Status::Pending)
}

const fn item_status_token(status: &Status) -> &'static str {
    match status {
        Status::Pending => "pending",
        Status::Uploading => "uploading",
        Status::Complete => "complete",
        Status::Failed(_) => "error",
        Status::Cancelled => "cancelled",
    }
}

fn format_file_size_with_locale(bytes: u64, _locale: &Locale) -> String {
    const KB: u64 = 1_024;
    const MB: u64 = KB * 1_024;

    if bytes >= MB {
        format!("{} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{} KB", bytes / KB)
    } else {
        format!("{bytes} bytes")
    }
}

fn apply_selected_files_plan(
    transition_to: Option<State>,
    raw: Vec<RawFile>,
    auto_upload: bool,
    before: impl FnOnce(&mut Context) + 'static,
) -> TransitionPlan<Machine> {
    let mut plan = if let Some(state) = transition_to {
        TransitionPlan::to(state)
    } else {
        TransitionPlan::new()
    };

    plan = plan.apply(move |context: &mut Context| {
        before(context);

        let (accepted, rejected) = validate_files(&raw, context);

        context.rejected_files = rejected;

        let mut current = context.files.get().clone();

        current.extend(accepted);

        context.files.set(current);
    });

    if auto_upload {
        plan = plan.then(Event::StartUpload);
    }

    plan
}

fn file_queue_plan(
    state: State,
    still_uploading: bool,
    apply: impl FnOnce(&mut Context) + 'static,
) -> TransitionPlan<Machine> {
    if matches!(state, State::Uploading) && !still_uploading {
        TransitionPlan::to(State::Idle).apply(apply)
    } else {
        TransitionPlan::context_only(apply)
    }
}

fn upload_finish_plan(
    still_uploading: bool,
    apply: impl FnOnce(&mut Context) + 'static,
) -> TransitionPlan<Machine> {
    if still_uploading {
        TransitionPlan::context_only(apply)
    } else {
        TransitionPlan::to(State::Idle).apply(apply)
    }
}

fn mark_pending_as_uploading(context: &mut Context) {
    let files = context.files.get().clone();

    let updated = files
        .into_iter()
        .map(|mut file| {
            if file.status == Status::Pending {
                file.status = Status::Uploading;
            }
            file
        })
        .collect();

    context.files.set(updated);
}

fn update_file_progress(context: &mut Context, file_id: &str, progress: f64) {
    let files = context.files.get().clone();

    let updated = files
        .into_iter()
        .map(|mut file| {
            if file.id == file_id {
                file.progress = progress;
            }
            file
        })
        .collect();

    context.files.set(updated);
}

fn upload_complete_updates(ctx: &Context, file_id: &str) -> bool {
    let files = projected_files_after_complete(ctx, file_id);

    files.iter().any(|file| file.status == Status::Uploading)
}

fn upload_error_updates(ctx: &Context, file_id: &str) -> bool {
    let files = projected_files_after_error(ctx, file_id, "");

    files.iter().any(|file| file.status == Status::Uploading)
}

fn remove_file_updates(ctx: &Context, file_id: &str) -> bool {
    let files = ctx
        .files
        .get()
        .iter()
        .filter(|file| file.id != file_id)
        .cloned()
        .collect::<Vec<_>>();

    files.iter().any(|file| file.status == Status::Uploading)
}

fn cancel_file_updates(ctx: &Context, file_id: &str) -> bool {
    let files = projected_files_after_cancel(ctx, file_id);

    files.iter().any(|file| file.status == Status::Uploading)
}

fn projected_files_after_complete(ctx: &Context, file_id: &str) -> Vec<Item> {
    ctx.files
        .get()
        .clone()
        .into_iter()
        .map(|mut file| {
            if file.id == file_id {
                file.status = Status::Complete;
                file.progress = 1.0;
            }

            file
        })
        .collect()
}

fn projected_files_after_error(ctx: &Context, file_id: &str, error: &str) -> Vec<Item> {
    ctx.files
        .get()
        .clone()
        .into_iter()
        .map(|mut file| {
            if file.id == file_id {
                file.status = Status::Failed(error.to_string());
                file.error = Some(error.to_string());
            }

            file
        })
        .collect()
}

fn projected_files_after_cancel(ctx: &Context, file_id: &str) -> Vec<Item> {
    ctx.files
        .get()
        .clone()
        .into_iter()
        .map(|mut file| {
            if file.id == file_id && file.status == Status::Uploading {
                file.status = Status::Cancelled;
            }

            file
        })
        .collect()
}

fn apply_upload_complete(context: &mut Context, file_id: &str) {
    context
        .files
        .set(projected_files_after_complete(context, file_id));
}

fn apply_upload_error(context: &mut Context, file_id: &str, error: &str) {
    context
        .files
        .set(projected_files_after_error(context, file_id, error));
}

fn remove_file_by_id(context: &mut Context, file_id: &str) {
    let updated = context
        .files
        .get()
        .iter()
        .filter(|file| file.id != file_id)
        .cloned()
        .collect();

    context.files.set(updated);
}

fn retry_file_by_id(context: &mut Context, file_id: &str) {
    let updated = context
        .files
        .get()
        .clone()
        .into_iter()
        .map(|mut file| {
            if file.id == file_id && matches!(file.status, Status::Failed(_)) {
                file.status = Status::Pending;
                file.progress = 0.0;
                file.error = None;
            }

            file
        })
        .collect();

    context.files.set(updated);
}

fn cancel_file_by_id(context: &mut Context, file_id: &str) {
    context
        .files
        .set(projected_files_after_cancel(context, file_id));
}

/// Validate raw files against context constraints.
fn validate_files(raw: &[RawFile], ctx: &Context) -> (Vec<Item>, Vec<Rejection>) {
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();

    let current_count = ctx.files.get().len();

    for file in raw {
        if let Some(max) = ctx.max_files
            && current_count + accepted.len() >= max
        {
            rejected.push(Rejection {
                name: file.name.clone(),
                size: file.size,
                mime_type: file.mime_type.clone(),
                reason: RejectionReason::TooMany,
            });

            continue;
        }

        if !ctx.accept.is_empty() && !mime_matches(&file.mime_type, &file.name, &ctx.accept) {
            rejected.push(Rejection {
                name: file.name.clone(),
                size: file.size,
                mime_type: file.mime_type.clone(),
                reason: RejectionReason::InvalidType,
            });

            continue;
        }

        if let Some(max_size) = ctx.max_file_size
            && file.size > max_size
        {
            rejected.push(Rejection {
                name: file.name.clone(),
                size: file.size,
                mime_type: file.mime_type.clone(),
                reason: RejectionReason::TooLarge,
            });

            continue;
        }

        if let Some(min_size) = ctx.min_file_size
            && file.size < min_size
        {
            rejected.push(Rejection {
                name: file.name.clone(),
                size: file.size,
                mime_type: file.mime_type.clone(),
                reason: RejectionReason::TooSmall,
            });

            continue;
        }

        accepted.push(Item {
            id: generate_file_id(ctx, &accepted),
            name: file.name.clone(),
            size: file.size,
            mime_type: file.mime_type.clone(),
            status: Status::Pending,
            progress: 0.0,
            error: None,
        });
    }

    (accepted, rejected)
}

fn generate_file_id(ctx: &Context, pending_accepted: &[Item]) -> String {
    let max_existing = ctx
        .files
        .get()
        .iter()
        .chain(pending_accepted.iter())
        .filter_map(|file| file.id.strip_prefix("file-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0);

    format!("file-{}", max_existing.saturating_add(1))
}

fn mime_matches(mime: &str, name: &str, patterns: &[String]) -> bool {
    let normalized = normalize_mime_type(mime);

    patterns.iter().any(|pattern| {
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 1];

            normalized.starts_with(prefix)
        } else if let Some(extension) = pattern.strip_prefix('.') {
            normalized == *pattern || file_name_has_extension(name, extension)
        } else {
            normalized == normalize_mime_type(pattern)
        }
    })
}

fn normalize_mime_type(mime_type: &str) -> String {
    let normalized = mime_type.trim().to_ascii_lowercase();

    if normalized == "image/jpg" {
        "image/jpeg".to_owned()
    } else {
        normalized
    }
}

fn file_name_has_extension(name: &str, extension: &str) -> bool {
    let Some(actual_extension) = name.rsplit_once('.').map(|(_, ext)| ext) else {
        return false;
    };

    !actual_extension.is_empty() && actual_extension.eq_ignore_ascii_case(extension)
}

#[cfg(test)]
mod tests;
