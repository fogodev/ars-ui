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
        let raw = if self.bytes_total == 0 {
            0.0
        } else {
            self.bytes_sent as f64 / self.bytes_total as f64
        };

        clamp_upload_fraction(raw)
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

    /// Reconcile the machine state with the currently visible file queue.
    ///
    /// Chained after [`Event::SetFiles`] so the resting state (`Idle` vs
    /// `Uploading`) tracks the queue revealed by the sync — whether that is a
    /// freshly controlled value or the internal value exposed when control is
    /// released.
    ReconcileState,
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

    /// Camera selection for mobile capture (`"user"` front, `"environment"` rear).
    pub capture: Option<String>,

    /// Name for form submission.
    pub name: Option<String>,

    /// Drag-over nesting counter (for nested elements).
    pub drag_counter: u32,

    /// Monotonic counter for the next generated file id.
    ///
    /// Only ever increases, so ids are never reused after files are removed —
    /// preventing late async upload events from being applied to a different
    /// file that happened to reuse an earlier id.
    pub next_file_id: u64,

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
///
/// Announcement variants drive the accessible live regions described in the
/// spec (§3.3, §4.2). On each, the adapter resolves the user-facing text from
/// the corresponding [`Messages`] field — using the carried `count` for the
/// pluralized add/reject messages — and writes it to the live region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter announces (assertive) that the dropzone became active, via
    /// [`Messages::dropzone_active`].
    AnnounceDropzoneActive,

    /// Adapter announces (assertive) that the dropzone is no longer active, via
    /// [`Messages::drop_zone_left`].
    AnnounceDropzoneLeft,

    /// Adapter announces (polite) that files were added, via
    /// [`Messages::files_added`] with `count`.
    AnnounceFilesAdded {
        /// Number of files accepted in this selection/drop.
        count: usize,
    },

    /// Adapter announces (polite) that files were rejected, via
    /// [`Messages::rejection_message`] with `count`.
    AnnounceFilesRejected {
        /// Number of files rejected in this selection/drop.
        count: usize,
    },

    /// Adapter announces (polite) that an upload completed.
    AnnounceUploadComplete,

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

        let next_file_id = next_id_after(files.get());

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
                capture: props.capture.clone(),
                name: props.name.clone(),
                drag_counter: 0,
                next_file_id,
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
        match event {
            Event::SetFiles(files) => {
                let files = files.clone();

                let apply = move |context: &mut Context| {
                    if let Some(files) = files {
                        context.next_file_id = context.next_file_id.max(next_id_after(&files));
                        context.files.set(files.clone());
                        context.files.sync_controlled(Some(files));
                    } else {
                        context.files.sync_controlled(None);
                    }
                };

                // Reconcile after the sync lands: chaining `ReconcileState` lets
                // the same path handle both a freshly controlled value and the
                // internal value revealed when control is released (`None`), which
                // is not visible until `sync_controlled` runs.
                return Some(TransitionPlan::context_only(apply).then(Event::ReconcileState));
            }

            Event::ReconcileState => {
                // `State::Uploading` must hold exactly when at least one file is
                // uploading. Drive the resting state between `Idle` and
                // `Uploading` to match the currently visible queue; `DragOver`'s
                // transient drag lifecycle is left untouched.
                return reconciled_state(*state, ctx.files.get()).map(TransitionPlan::to);
            }

            Event::SetProps => {
                // Becoming disabled/read-only while a drag is in progress would
                // otherwise trap the machine: the guard below swallows the
                // `DragLeave`/`Drop` that would normally clear `dragging`, so the
                // dropzone would stay stuck in its drag-over UI/ARIA state. Reset
                // to `Idle` and clear the drag flags as part of the sync.
                let reset_drag =
                    (props.disabled || props.readonly) && matches!(state, State::DragOver);

                let apply = {
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
                    let capture = props.capture.clone();
                    let name = props.name.clone();

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
                        context.capture = capture;
                        context.name = name;

                        if reset_drag {
                            context.dragging = false;
                            context.drag_counter = 0;
                        }
                    }
                };

                return Some(if reset_drag {
                    TransitionPlan::to(State::Idle).apply(apply)
                } else {
                    TransitionPlan::context_only(apply)
                });
            }

            Event::UploadProgress { file_id, progress } if matches!(state, State::Uploading) => {
                let file_id = file_id.clone();
                let progress = clamp_upload_fraction(*progress);
                return Some(TransitionPlan::context_only(
                    move |context: &mut Context| {
                        update_file_progress(context, &file_id, progress);
                    },
                ));
            }

            Event::UploadComplete { file_id } if matches!(state, State::Uploading) => {
                let file_id = file_id.clone();
                // Only announce when this event actually completes a file that
                // was still uploading (a late duplicate for a terminal file is a
                // no-op and must stay silent).
                let did_complete = ctx
                    .files
                    .get()
                    .iter()
                    .any(|file| file.id == file_id && file.status == Status::Uploading);
                let still_uploading = upload_complete_updates(ctx, &file_id);
                let plan = upload_finish_plan(still_uploading, move |context: &mut Context| {
                    apply_upload_complete(context, &file_id);
                });
                return Some(if did_complete {
                    plan.with_effect(PendingEffect::named(Effect::AnnounceUploadComplete))
                } else {
                    plan
                });
            }

            Event::UploadError { file_id, error } if matches!(state, State::Uploading) => {
                let file_id = file_id.clone();
                let error = error.clone();
                let still_uploading = upload_error_updates(ctx, &file_id);
                return Some(upload_finish_plan(
                    still_uploading,
                    move |context: &mut Context| {
                        apply_upload_error(context, &file_id, &error);
                    },
                ));
            }

            _ => {}
        }

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
                        TransitionPlan::to(State::Idle)
                            .apply(|context: &mut Context| {
                                context.drag_counter = 0;
                                context.dragging = false;
                            })
                            .with_effect(PendingEffect::named(Effect::AnnounceDropzoneLeft)),
                    )
                } else {
                    Some(TransitionPlan::context_only(
                        move |context: &mut Context| {
                            context.drag_counter = new_counter;
                        },
                    ))
                }
            }

            (State::DragOver, Event::Drop(raw_files)) => Some(apply_selected_files_plan(
                Some(State::Idle),
                raw_files,
                ctx.auto_upload,
                ctx,
                |context: &mut Context| {
                    context.dragging = false;
                    context.drag_counter = 0;
                },
            )),

            (State::Idle, Event::FilesSelected(raw_files))
            | (State::Uploading, Event::FilesSelected(raw_files)) => {
                let auto_upload = ctx.auto_upload;
                Some(apply_selected_files_plan(
                    None,
                    raw_files,
                    auto_upload,
                    ctx,
                    |_| {},
                ))
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

            (State::Uploading, Event::StartUpload) => {
                if !has_pending_files(ctx, props) {
                    return None;
                }

                Some(TransitionPlan::context_only(|context: &mut Context| {
                    mark_pending_as_uploading(context);
                }))
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
            || old.capture != new.capture
            || old.name != new.name
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

        if self.ctx.disabled || self.ctx.readonly {
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
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.trigger_label)(&self.ctx.locale),
            );

        if self.ctx.disabled || self.ctx.readonly {
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

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button");

        if let Some(file) = self.ctx.files.get().get(index) {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.remove_label)(&file.name, &self.ctx.locale),
            );
        }

        // The transition guard ignores `RemoveFile` while disabled/read-only, so
        // the delete control must read as inert too — matching the trigger and
        // hidden input — rather than a focusable button that silently does nothing.
        if self.ctx.disabled || self.ctx.readonly {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
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

        if let Some(ref capture) = self.ctx.capture {
            attrs.set(HtmlAttr::Capture, capture);
        }

        if let Some(ref name) = self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }

        if self.ctx.required && self.ctx.files.get().is_empty() {
            attrs.set_bool(HtmlAttr::Required, true);
        }

        if self.ctx.disabled || self.ctx.readonly {
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

    /// Dispatches keyboard intent on the file item at `index`.
    ///
    /// Per the spec keyboard contract, <kbd>Delete</kbd>/<kbd>Backspace</kbd> on
    /// a focused file item removes that file. The adapter passes the item's index
    /// so the core can resolve the file id without tracking per-item focus.
    pub fn on_item_keydown(&self, index: usize, data: &KeyboardEventData) {
        if matches!(data.key, KeyboardKey::Delete | KeyboardKey::Backspace) {
            self.on_item_delete_trigger_click(index);
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

fn format_file_size_with_locale(bytes: u64, locale: &Locale) -> String {
    use alloc::format;

    use ars_i18n::number::{FormatOptions, Formatter};

    const KB: f64 = 1_024.0;
    const MB: f64 = KB * 1_024.0;
    let bytes_f = bytes as f64;

    let (value, suffix) = if bytes_f >= MB {
        (bytes_f / MB, "MB")
    } else if bytes_f >= KB {
        (bytes_f / KB, "KB")
    } else {
        (bytes_f, "bytes")
    };

    let max_fraction_digits = if suffix == "bytes" || value.fract() == 0.0 {
        0
    } else {
        1
    };

    let number = Formatter::new(
        locale,
        FormatOptions {
            max_fraction_digits,
            ..FormatOptions::default()
        },
    )
    .format(value);

    format!("{number} {suffix}")
}

fn apply_selected_files_plan(
    transition_to: Option<State>,
    raw: &[RawFile],
    auto_upload: bool,
    ctx: &Context,
    before: impl FnOnce(&mut Context) + 'static,
) -> TransitionPlan<Machine> {
    // Validate up front (the `before` hook only touches drag state, never the
    // fields validation reads) so the accepted/rejected counts are known when
    // the announcement effects are attached — effects are fixed before `apply`
    // runs.
    let mut next_file_id = ctx.next_file_id;
    let (accepted, rejected) = validate_files(raw, ctx, &mut next_file_id);
    let accepted_count = accepted.len();
    let rejected_count = rejected.len();

    let mut plan = if let Some(state) = transition_to {
        TransitionPlan::to(state)
    } else {
        TransitionPlan::new()
    };

    plan = plan.apply(move |context: &mut Context| {
        before(context);

        context.next_file_id = next_file_id;
        context.rejected_files = rejected;

        let mut current = context.files.get().clone();

        current.extend(accepted);

        context.files.set(current);
    });

    if accepted_count > 0 {
        plan = plan.with_effect(PendingEffect::named(Effect::AnnounceFilesAdded {
            count: accepted_count,
        }));
    }

    if rejected_count > 0 {
        plan = plan.with_effect(PendingEffect::named(Effect::AnnounceFilesRejected {
            count: rejected_count,
        }));
    }

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

#[expect(clippy::missing_const_for_fn, reason = "f64::is_nan is not const")]
fn clamp_upload_fraction(progress: f64) -> f64 {
    if progress.is_nan() {
        0.0
    } else {
        progress.clamp(0.0, 1.0)
    }
}

fn update_file_progress(context: &mut Context, file_id: &str, progress: f64) {
    let progress = clamp_upload_fraction(progress);
    let files = context.files.get().clone();

    let updated = files
        .into_iter()
        .map(|mut file| {
            // Only the actively uploading file accepts progress: a late callback
            // for a file that was already cancelled/failed/completed must not
            // mutate its terminal progress.
            if file.id == file_id && file.status == Status::Uploading {
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
            if file.id == file_id && file.status == Status::Uploading {
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
            if file.id == file_id && file.status == Status::Uploading {
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
///
/// `next_file_id` is the machine's monotonic id counter; each accepted file
/// consumes the next value and advances it so ids are never reused.
fn validate_files(
    raw: &[RawFile],
    ctx: &Context,
    next_file_id: &mut u64,
) -> (Vec<Item>, Vec<Rejection>) {
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();

    let current_count = ctx.files.get().len();

    for file in raw {
        if !ctx.multiple && (current_count >= 1 || !accepted.is_empty()) {
            rejected.push(Rejection {
                name: file.name.clone(),
                size: file.size,
                mime_type: file.mime_type.clone(),
                reason: RejectionReason::TooMany,
            });

            continue;
        }

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
            id: generate_file_id(next_file_id),
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

/// Consumes the next monotonic file id and advances the counter.
fn generate_file_id(next_file_id: &mut u64) -> String {
    let id = *next_file_id;

    *next_file_id = next_file_id.saturating_add(1);

    format!("file-{id}")
}

/// The next monotonic id that sits strictly above every `file-N` id in `files`.
///
/// Used to seed [`Context::next_file_id`] at init and to advance it past
/// externally supplied ids on [`Event::SetFiles`].
fn next_id_after(files: &[Item]) -> u64 {
    files
        .iter()
        .filter_map(|file| file.id.strip_prefix("file-")?.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

/// Reconciles the machine state with the currently visible file set.
///
/// `State::Uploading` must hold exactly when at least one file is uploading, so
/// when [`Event::ReconcileState`] runs after a [`Event::SetFiles`] sync it drives
/// the resting state between [`State::Idle`] and [`State::Uploading`] to match
/// the visible queue. Returns `None` when no state change is required (including
/// while in [`State::DragOver`], whose transient drag lifecycle is left untouched).
fn reconciled_state(current: State, files: &[Item]) -> Option<State> {
    let any_uploading = files.iter().any(|file| file.status == Status::Uploading);

    match (current, any_uploading) {
        (State::Uploading, false) => Some(State::Idle),
        (State::Idle, true) => Some(State::Uploading),
        _ => None,
    }
}

fn mime_matches(mime: &str, name: &str, patterns: &[String]) -> bool {
    let normalized = normalize_mime_type(mime);

    patterns.iter().any(|pattern| {
        let pattern = pattern.trim();

        if pattern.ends_with("/*") {
            let prefix = normalize_mime_type(&pattern[..pattern.len() - 1]);

            normalized.starts_with(&prefix)
        } else if let Some(extension) = pattern.strip_prefix('.') {
            normalized == normalize_mime_type(pattern) || file_name_has_extension(name, extension)
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
    if extension.is_empty() {
        return false;
    }

    // Match the full dotted suffix so compound extensions (e.g. `.tar.gz`) match
    // `archive.tar.gz`, not just the segment after the final dot. Browser
    // `accept` extension tokens are matched case-insensitively against the
    // filename suffix.
    let suffix = format!(".{}", extension.to_ascii_lowercase());

    name.to_ascii_lowercase().ends_with(&suffix)
}

#[cfg(test)]
mod tests;
