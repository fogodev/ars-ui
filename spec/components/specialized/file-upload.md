---
component: FileUpload
category: specialized
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
  ark-ui: FileUpload
---

# FileUpload

A `FileUpload` component provides drag-and-drop file selection, click-to-browse, file
validation (type, size, count), and optional upload progress tracking. It renders a
dropzone area, a file list, and integrates with the native `<input type="file">`.

```rust
// crates/ars-core/src/components/file_upload.rs

use crate::{Bindable, ComponentId};
use crate::machine::{Machine, TransitionPlan, ComponentIds, AttrMap};

/// Represents a single file in the upload queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Unique identifier for this file item.
    pub id: String,
    /// Original file name.
    pub name: String,
    /// File size in bytes.
    pub size: u64,
    /// MIME type (e.g., "image/png").
    pub mime_type: String,
    /// Current upload status.
    pub status: Status,
    /// Upload progress as a fraction [0.0, 1.0].
    pub progress: f64,
    /// Error message if status is Error.
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
///
/// Reported via `on_progress: Callback<file_upload::Progress>`.
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
    /// Progress as a fraction in [0.0, 1.0].
    pub fn fraction(&self) -> f64 {
        if self.bytes_total == 0 { 0.0 }
        else { self.bytes_sent as f64 / self.bytes_total as f64 }
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
```

## 1. State Machine

### 1.1 States

```rust
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
```

### 1.2 Events

```rust
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
    /// Retry a failed upload via `retry_upload(file_index)`.
    RetryFile {
        /// The id of the file.
        file_id: String,
    },
    /// Cancel an in-progress upload via `cancel_upload(file_index)`.
    /// Fires `on_cancel` callback with the file index.
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
```

### 1.3 Context

```rust
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
    /// Accepted MIME types (e.g., ["image/*", "application/pdf"]).
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
    /// Component instance IDs.
    pub id: ComponentId,
    /// The id of the dropzone.
    pub dropzone_id: String,
    /// The id of the input.
    pub input_id: String,
    /// The id of the label.
    pub label_id: String,
    /// The id of the file list.
    pub file_list_id: String,
}
```

### 1.4 Props

```rust
/// The props for the `FileUpload` component.
#[derive(Clone, Debug, PartialEq, HasId)]
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
    /// Locale override. When `None`, resolved via `resolve_locale()`.
    pub locale: Option<Locale>,
    /// Translatable messages. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            files: None,
            default_files: Vec::new(),
            disabled: false,
            readonly: false,
            required: false,
            multiple: false,
            capture: None,
            accept: Vec::new(),
            max_file_size: None,
            min_file_size: None,
            max_files: None,
            auto_upload: false,
            directory: false,
            name: None,
            id: String::new(),
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Guards

```rust
/// Whether the component is disabled.
fn is_disabled(ctx: &Context, _props: &Props) -> bool {
    ctx.disabled
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
    ctx.files.get().iter().any(|f| f.status == Status::Pending)
}

/// Whether there are any uploading files.
fn has_uploading_files(ctx: &Context, _props: &Props) -> bool {
    ctx.files.get().iter().any(|f| f.status == Status::Uploading)
}
```

### 1.6 File Validation

```rust
/// Validate a set of raw files against the constraints, returning accepted and rejected.
fn validate_files(
    raw: &[RawFile],
    ctx: &Context,
) -> (Vec<Item>, Vec<Rejection>) {
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let current_count = ctx.files.get().len();

    for (i, file) in raw.iter().enumerate() {
        // Check max files
        if let Some(max) = ctx.max_files {
            if current_count + accepted.len() >= max {
                rejected.push(Rejection {
                    name: file.name.clone(),
                    size: file.size,
                    mime_type: file.mime_type.clone(),
                    reason: RejectionReason::TooMany,
                });
                continue;
            }
        }

        // Check MIME type
        if !ctx.accept.is_empty() && !mime_matches(&file.mime_type, &ctx.accept) {
            rejected.push(Rejection {
                name: file.name.clone(),
                size: file.size,
                mime_type: file.mime_type.clone(),
                reason: RejectionReason::InvalidType,
            });
            continue;
        }

        // Check file size
        if let Some(max_size) = ctx.max_file_size {
            if file.size > max_size {
                rejected.push(Rejection {
                    name: file.name.clone(),
                    size: file.size,
                    mime_type: file.mime_type.clone(),
                    reason: RejectionReason::TooLarge,
                });
                continue;
            }
        }

        if let Some(min_size) = ctx.min_file_size {
            if file.size < min_size {
                rejected.push(Rejection {
                    name: file.name.clone(),
                    size: file.size,
                    mime_type: file.mime_type.clone(),
                    reason: RejectionReason::TooSmall,
                });
                continue;
            }
        }

        accepted.push(Item {
            id: generate_file_id(),
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

fn mime_matches(mime: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| {
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 1];
            mime.starts_with(prefix)
        } else if pattern.starts_with('.') {
            // Extension-based: ".pdf", ".png"
            mime.ends_with(pattern) || mime_from_extension(pattern).map_or(false, |m| m == mime)
        } else {
            mime == pattern
        }
    })
}
```

### 1.7 Full Machine Implementation

```rust
/// The machine for the `FileUpload` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let files = match &props.files {
            Some(f) => Bindable::controlled(f.clone()),
            None => Bindable::uncontrolled(props.default_files.clone()),
        };

        let ids = ComponentIds::from_id(&props.id);
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);

        (State::Idle, Context {
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
            locale,
            messages,
            id: ids.id().to_string(),
            dropzone_id: ids.part("dropzone"),
            input_id: ids.part("input"),
            label_id: ids.part("label"),
            file_list_id: ids.part("file-list"),
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled || ctx.readonly {
            return match event {
                Event::Focus { part } => {
                    let part = *part;
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.focused_part = Some(part);
                    }))
                }
                Event::Blur { .. } => {
                    Some(TransitionPlan::context_only(|ctx| {
                        ctx.focused_part = None;
                    }))
                }
                _ => None,
            };
        }

        match (state, event) {
            // --- Drag events ---
            (State::Idle, Event::DragEnter) => {
                Some(TransitionPlan::to(State::DragOver).apply(|ctx| {
                    ctx.dragging = true;
                    ctx.drag_counter = 1;
                }).with_effect(PendingEffect::new("announce-dropzone-active", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    platform.announce(&(ctx.messages.dropzone_active)(&ctx.locale));
                    no_cleanup()
                })))
            }

            (State::DragOver, Event::DragEnter) => {
                // Increment counter for nested elements
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.drag_counter += 1;
                }))
            }

            (State::DragOver, Event::DragOver) => {
                // Stay in DragOver, prevent default (handled by the adapter)
                None
            }

            (State::DragOver, Event::DragLeave) => {
                // Decrement counter; only leave DragOver when counter hits 0
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.drag_counter = ctx.drag_counter.saturating_sub(1);
                    if ctx.drag_counter == 0 {
                        ctx.dragging = false;
                    }
                }))
                // Note: actual state change to Idle happens in a follow-up check.
                // For simplicity, we handle this in the transition:
            }

            (State::DragOver, Event::Drop(raw_files)) => {
                let raw = raw_files.clone();
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.dragging = false;
                    ctx.drag_counter = 0;
                    let (accepted, rejected) = validate_files(&raw, ctx);
                    ctx.rejected_files = rejected;
                    let mut current = ctx.files.get().clone();
                    current.extend(accepted);
                    ctx.files.set(current);
                }))
            }

            // --- File selection via input ---
            (State::Idle, Event::FilesSelected(raw_files))
            | (State::Uploading, Event::FilesSelected(raw_files)) => {
                let raw = raw_files.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let (accepted, rejected) = validate_files(&raw, ctx);
                    ctx.rejected_files = rejected;
                    let mut current = ctx.files.get().clone();
                    current.extend(accepted);
                    ctx.files.set(current);
                    // If auto_upload, chain a StartUpload event
                }))
            }

            // --- Upload lifecycle ---
            (State::Idle, Event::StartUpload) => {
                if !has_pending_files(ctx, props) { return None; }
                Some(TransitionPlan::to(State::Uploading).apply(|ctx| {
                    let files = ctx.files.get().clone();
                    let updated: Vec<Item> = files.into_iter().map(|mut f| {
                        if f.status == Status::Pending {
                            f.status = Status::Uploading;
                        }
                        f
                    }).collect();
                    ctx.files.set(updated);
                }))
            }

            (State::Uploading, Event::UploadProgress { file_id, progress }) => {
                let fid = file_id.clone();
                let prog = *progress;
                Some(TransitionPlan::context_only(move |ctx| {
                    let files = ctx.files.get().clone();
                    let updated: Vec<Item> = files.into_iter().map(|mut f| {
                        if f.id == fid {
                            f.progress = prog;
                        }
                        f
                    }).collect();
                    ctx.files.set(updated);
                }))
            }

            (State::Uploading, Event::UploadComplete { file_id }) => {
                let fid = file_id.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let files = ctx.files.get().clone();
                    let updated: Vec<Item> = files.into_iter().map(|mut f| {
                        if f.id == fid {
                            f.status = Status::Complete;
                            f.progress = 1.0;
                        }
                        f
                    }).collect();
                    ctx.files.set(updated);
                }))
                // The Service should check after this action whether any files
                // are still Uploading. If none, transition to Idle.
            }

            (State::Uploading, Event::UploadError { file_id, error }) => {
                let fid = file_id.clone();
                let err = error.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let files = ctx.files.get().clone();
                    let updated: Vec<Item> = files.into_iter().map(|mut f| {
                        if f.id == fid {
                            f.status = Status::Failed(err.clone());
                            f.error = Some(err.clone());
                        }
                        f
                    }).collect();
                    ctx.files.set(updated);
                }))
            }

            // --- File management ---
            (_, Event::RemoveFile { file_id }) => {
                let fid = file_id.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let files = ctx.files.get().clone();
                    let updated: Vec<Item> = files.into_iter()
                        .filter(|f| f.id != fid)
                        .collect();
                    ctx.files.set(updated);
                }))
            }

            (_, Event::ClearFiles) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.files.set(Vec::new());
                    ctx.rejected_files.clear();
                }))
            }

            (_, Event::RetryFile { file_id }) => {
                let fid = file_id.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let files = ctx.files.get().clone();
                    let updated: Vec<Item> = files.into_iter().map(|mut f| {
                        if f.id == fid && matches!(f.status, Status::Failed(_)) {
                            f.status = Status::Pending;
                            f.progress = 0.0;
                            f.error = None;
                        }
                        f
                    }).collect();
                    ctx.files.set(updated);
                }))
            }

            (_, Event::OpenFilePicker) => {
                Some(TransitionPlan::context_only(|_ctx| {
                }).with_named_effect("open-file-picker", |ctx, _props, _send| {
                    let input_id = ctx.input_id.clone();
                    trigger_click_on_element(&input_id);
                    no_cleanup()
                }))
            }

            (_, Event::Focus { part }) => {
                let part = *part;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_part = Some(part);
                }))
            }

            (_, Event::Blur { .. }) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused_part = None;
                }))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "file-upload"]
pub enum Part {
    Root,
    Label,
    Dropzone,
    Trigger,
    ItemGroup,
    Item { index: usize },
    ItemName { index: usize },
    ItemSizeText { index: usize },
    ItemDeleteTrigger { index: usize },
    ItemProgress { index: usize },
    HiddenInput,
}

/// The API for the `FileUpload` component.
pub struct Api<'a> {
    /// The state of the component.
    state: &'a State,
    /// The context of the component.
    ctx: &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    // --- Computed state ---

    /// Whether the component is currently dragging.
    pub fn is_dragging(&self) -> bool { self.ctx.dragging }

    /// Whether the component is currently uploading.
    pub fn is_uploading(&self) -> bool { *self.state == State::Uploading }

    /// The files of the component.
    pub fn files(&self) -> &[Item] { self.ctx.files.get() }

    /// The rejected files of the component.
    pub fn rejected_files(&self) -> &[Rejection] { &self.ctx.rejected_files }

    /// Whether the maximum number of files has been reached.
    pub fn is_max_files_reached(&self) -> bool { is_max_files_reached(self.ctx, self.props) }

    /// Human-readable rejection announcement for screen readers.
    pub fn rejection_message(&self) -> Option<String> {
        if self.ctx.rejected_files.is_empty() {
            None
        } else {
            Some((self.ctx.messages.rejection_message)(self.ctx.rejected_files.len(), &self.ctx.locale))
        }
    }

    // --- Part attrs ---

    /// The attrs for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.dropzone_label)(&self.ctx.locale));
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

    /// The attrs for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.label_id);
        attrs.set(HtmlAttr::For, &self.ctx.input_id);
        attrs
    }

    /// The attrs for the dropzone element.
    pub fn dropzone_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Dropzone.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.dropzone_id);
        attrs.set(HtmlAttr::Role, "button");
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::DragOver => "drag-over",
            State::Uploading => "uploading",
        });
        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), &self.ctx.label_id);
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        // Event handlers (drag/drop, click, keydown for file picking) are typed methods on the Api struct.
        attrs
    }

    /// The attrs for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.trigger_label)(&self.ctx.locale));
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        // Event handlers (click to open file picker) are typed methods on the Api struct.
        attrs
    }

    /// The attrs for the item group (file list container).
    pub fn item_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.file_list_id);
        attrs.set(HtmlAttr::Role, "list");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.file_list_label)(&self.ctx.locale));
        attrs
    }

    /// The attrs for a file item element at the given index.
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Item { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let files = self.ctx.files.get();
        if let Some(file) = files.get(index) {
            attrs.set(HtmlAttr::Role, "listitem");
            attrs.set(HtmlAttr::Data("ars-state"), match file.status {
                Status::Pending => "pending",
                Status::Uploading => "uploading",
                Status::Complete => "complete",
                Status::Failed(_) => "error",
            });
            attrs.set(HtmlAttr::Data("ars-file-id"), &file.id);
            attrs.set(HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.file_size)(file.size, &self.ctx.locale));
        }
        attrs
    }

    /// The attrs for the file item name display at the given index.
    pub fn item_name_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemName { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attrs for the file item size text at the given index.
    pub fn item_size_text_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemSizeText { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attrs for the file item delete trigger at the given index.
    pub fn item_delete_trigger_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemDeleteTrigger { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let files = self.ctx.files.get();
        if let Some(file) = files.get(index) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.remove_label)(&file.name, &self.ctx.locale));
        }
        // Event handlers (click to remove file) are typed methods on the Api struct.
        attrs
    }

    /// The attrs for the file item progress indicator at the given index.
    pub fn item_progress_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemProgress { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attrs for the hidden input element.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.input_id);
        attrs.set(HtmlAttr::Type, "file");
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Class, "ars-sr-input");
        if self.ctx.multiple {
            attrs.set_bool(HtmlAttr::Multiple, true);
        }
        if !self.ctx.accept.is_empty() {
            attrs.set(HtmlAttr::Accept, self.ctx.accept.join(","));
        }
        if self.ctx.directory {
            attrs.set(HtmlAttr::WebkitDirectory, "");
        }
        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        // Event handlers (change for file selection) are typed methods on the Api struct.
        attrs
    }

    /// Returns a human-readable validation error string for the given rejection reason.
    /// Adapters can use this to display per-file error messages in the file list.
    pub fn validation_error_text(&self, rejection: &Rejection) -> String {
        match &rejection.reason {
            RejectionReason::TooLarge => {
                (self.ctx.messages.too_large)(
                    self.ctx.max_file_size.unwrap_or(0),
                    &self.ctx.locale,
                )
            }
            RejectionReason::InvalidType => {
                (self.ctx.messages.wrong_type)(&self.ctx.locale)
            }
            RejectionReason::TooMany => {
                (self.ctx.messages.too_many_files)(&self.ctx.locale)
            }
            RejectionReason::TooSmall => {
                (self.ctx.messages.too_small)(
                    self.ctx.min_file_size.unwrap_or(0),
                    &self.ctx.locale,
                )
            }
            RejectionReason::CustomValidation(msg) => msg.clone(),
        }
    }

    // --- Imperative actions ---

    /// Open the file picker.
    pub fn open_file_picker(&self) { (self.send)(Event::OpenFilePicker); }

    /// Start uploading the files.
    pub fn start_upload(&self) { (self.send)(Event::StartUpload); }

    /// Clear the files.
    pub fn clear_files(&self) { (self.send)(Event::ClearFiles); }

    /// Remove a file.
    pub fn remove_file(&self, file_id: &str) {
        (self.send)(Event::RemoveFile { file_id: file_id.to_string() });
    }

    /// Retry a failed upload.
    pub fn retry_file(&self, file_id: &str) {
        (self.send)(Event::RetryFile { file_id: file_id.to_string() });
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
```

## 2. Anatomy

```text
FileUpload
├── Root
├── Label              (describes the upload area)
├── Dropzone           (drag-and-drop target area)
├── Trigger            (button to open file picker)
├── ItemGroup          (container for file items)
│   └── Item × N
│       ├── ItemName          (file name display)
│       ├── ItemSizeText      (formatted file size)
│       ├── ItemProgress      (upload progress indicator)
│       └── ItemDeleteTrigger (remove button)
└── HiddenInput        (native <input type="file">)
```

| Part              | Element               | Key Attributes                                          |
| ----------------- | --------------------- | ------------------------------------------------------- |
| Root              | `<div>`               | `data-ars-scope`, `data-ars-part`, `aria-label`         |
| Label             | `<label>`             | `id`, `for`                                             |
| Dropzone          | `<div>`               | `role="button"`, `tabindex="0"`, `aria-labelledby`      |
| Trigger           | `<button>`            | `aria-label`                                            |
| ItemGroup         | `<ul>`                | `role="list"`, `aria-label`                             |
| Item              | `<li>`                | `role="listitem"`, `data-ars-state`, `data-ars-file-id` |
| ItemName          | `<span>`              | file name text                                          |
| ItemSizeText      | `<span>`              | formatted file size                                     |
| ItemDeleteTrigger | `<button>`            | `aria-label="Remove {filename}"`                        |
| ItemProgress      | `<div>`               | upload progress indicator                               |
| HiddenInput       | `<input type="file">` | `tabindex="-1"`, `multiple`, `accept`                   |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute / Behaviour  | Element           | Value                      |
| ---------------------- | ----------------- | -------------------------- |
| `role="button"`        | Dropzone          | Clickable drop area        |
| `tabindex="0"`         | Dropzone          | Keyboard focusable         |
| `aria-labelledby`      | Dropzone          | Label ID                   |
| `aria-disabled="true"` | Dropzone, Trigger | When disabled              |
| `role="list"`          | ItemGroup         | Semantic list              |
| `role="listitem"`      | Item              | Semantic list item         |
| `aria-label`           | Trigger           | `"Choose files to upload"` |
| `aria-label`           | ItemDeleteTrigger | `"Remove {filename}"`      |
| `aria-label`           | ItemGroup         | `"Uploaded files"`         |

### 3.2 Keyboard Interaction

| Key              | Element        | Action                      |
| ---------------- | -------------- | --------------------------- |
| Enter / Space    | Dropzone       | Open file picker            |
| Tab              | File items     | Navigate between file items |
| Delete/Backspace | Item (focused) | Remove file                 |

### 3.3 Screen Reader Announcements

The FileUpload component uses an `aria-live="polite"` region to announce file additions, rejections, and upload completions to screen reader users.

For drag-and-drop state changes, a separate `aria-live="assertive"` live region is used so that screen reader users are informed of drop zone activity. See also [4.2 Drag-and-Drop Live Region Announcements](#42-drag-and-drop-live-region-announcements).

The adapter MUST:

1. Insert a visually-hidden `<div aria-live="assertive" aria-atomic="true">` as
   a sibling of the dropzone.
2. On `DragEnter` transition, set its text content to `messages.drop_zone_active`.
3. On `DragLeave` (when `drag_counter` reaches 0), set text to `messages.drop_zone_left`.
4. On `Drop` transition, set text to `(messages.files_added)(accepted_count)`.
5. Clear the live region text after a 3-second timeout to avoid stale announcements.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    pub dropzone_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub dropzone_active: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub file_list_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub remove_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    pub rejection_message: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    pub file_size: MessageFn<dyn Fn(u64, &Locale) -> String + Send + Sync>,
    pub too_large: MessageFn<dyn Fn(u64, &Locale) -> String + Send + Sync>,
    pub wrong_type: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub too_many_files: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    pub too_small: MessageFn<dyn Fn(u64, &Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            dropzone_label: MessageFn::static_str("Drag and drop files here, or click to browse"),
            dropzone_active: MessageFn::static_str("Drop files to upload"),
            trigger_label: MessageFn::static_str("Browse files"),
            file_list_label: MessageFn::static_str("Uploaded files"),
            remove_label: MessageFn::new(|name, _locale| format!("Remove {}", name)),
            rejection_message: MessageFn::new(|count, _locale| format!("{} files rejected", count)),
            file_size: MessageFn::new(|bytes, locale| format_file_size(bytes, locale)),
            too_large: MessageFn::new(|max, locale| format!("File exceeds maximum size of {}", format_file_size(max, locale))),
            wrong_type: MessageFn::static_str("File type not accepted"),
            too_many_files: MessageFn::static_str("Too many files selected"),
            too_small: MessageFn::new(|min, locale| format!("File below minimum size of {}", format_file_size(min, locale))),
        }
    }
}
impl ComponentMessages for Messages {}
```

| Key                              | Default (en-US)                     | Purpose                     |
| -------------------------------- | ----------------------------------- | --------------------------- |
| `file_upload.label`              | `"Upload files"`                    | Default label text          |
| `file_upload.trigger_label`      | `"Choose files to upload"`          | Trigger button label        |
| `file_upload.dropzone_text`      | `"Drag and drop files here"`        | Dropzone instructional text |
| `file_upload.remove_label`       | `"Remove {filename}"`               | Remove button label         |
| `file_upload.retry_label`        | `"Retry upload of {filename}"`      | Retry button label          |
| `file_upload.file_list_label`    | `"Uploaded files"`                  | File list label             |
| `file_upload.size_bytes`         | `"{size} bytes"`                    | File size (bytes)           |
| `file_upload.size_kb`            | `"{size} KB"`                       | File size (kilobytes)       |
| `file_upload.size_mb`            | `"{size} MB"`                       | File size (megabytes)       |
| `file_upload.error_too_large`    | `"File too large"`                  | Rejection: size exceeded    |
| `file_upload.error_too_small`    | `"File too small"`                  | Rejection: minimum size     |
| `file_upload.error_invalid_type` | `"File type not accepted"`          | Rejection: MIME type        |
| `file_upload.error_too_many`     | `"Maximum number of files reached"` | Rejection: count            |
| `file_upload.status_pending`     | `"Pending"`                         | Status label                |
| `file_upload.status_uploading`   | `"Uploading..."`                    | Status label                |
| `file_upload.status_complete`    | `"Complete"`                        | Status label                |
| `file_upload.status_error`       | `"Error"`                           | Status label                |

- **File size formatting**: Uses locale-aware number formatting via `ars-i18n` for
  decimal separators and unit formatting (e.g., `"1,5 MB"` in de-DE vs `"1.5 MB"` in
  en-US).
- **RTL**: The dropzone layout, file list, and progress bars flow correctly in RTL.

### 4.2 Drag-and-Drop Live Region Announcements

The FileUpload component MUST announce drag-and-drop state changes via an
`aria-live="assertive"` live region so that screen reader users are informed of
drop zone activity. The adapter populates the live region text from
`Messages` fields:

| DnD Event   | Live Region Text Source         | Default (en-US)                   |
| ----------- | ------------------------------- | --------------------------------- |
| `dragenter` | `messages.drop_zone_active`     | `"Drop zone is active"`           |
| `dragleave` | `messages.drop_zone_left`       | `"Drop zone is no longer active"` |
| `drop`      | `(messages.files_added)(count)` | `"{N} files added"`               |

## 5. Library Parity

> Compared against: Ark UI (`FileUpload`).

### 5.1 Props

| Feature                  | ars-ui                    | Ark UI                                   | Notes                                                                  |
| ------------------------ | ------------------------- | ---------------------------------------- | ---------------------------------------------------------------------- |
| `accept`                 | `accept`                  | `accept`                                 | Equivalent                                                             |
| `files` / `defaultFiles` | `files` / `default_files` | `acceptedFiles` / `defaultAcceptedFiles` | Equivalent (different naming)                                          |
| `allowDrop`              | (always true)             | `allowDrop`                              | ars-ui always allows drop                                              |
| `capture`                | `capture`                 | `capture`                                | Equivalent                                                             |
| `directory`              | `directory`               | `directory`                              | Equivalent                                                             |
| `disabled`               | `disabled`                | `disabled`                               | Equivalent                                                             |
| `readOnly`               | `readonly`                | `readOnly`                               | Equivalent                                                             |
| `required`               | `required`                | `required`                               | Equivalent                                                             |
| `invalid`                | --                        | `invalid`                                | Ark-only; ars-ui validates at form level                               |
| `maxFiles`               | `max_files`               | `maxFiles`                               | Equivalent                                                             |
| `maxFileSize`            | `max_file_size`           | `maxFileSize`                            | Equivalent                                                             |
| `minFileSize`            | `min_file_size`           | `minFileSize`                            | Equivalent                                                             |
| `multiple`               | `multiple`                | (via maxFiles > 1)                       | Equivalent intent                                                      |
| `name`                   | `name`                    | `name`                                   | Equivalent                                                             |
| `locale`                 | `locale`                  | `locale`                                 | Equivalent                                                             |
| `preventDocumentDrop`    | --                        | `preventDocumentDrop`                    | Ark prevents accidental navigation; adapter concern                    |
| `transformFiles`         | --                        | `transformFiles`                         | Ark transforms files before accepting; niche                           |
| `validate`               | --                        | `validate`                               | Ark custom validation; ars-ui uses `RejectionReason::CustomValidation` |

**Gaps:** None worth adopting. `preventDocumentDrop` is an adapter-level browser event concern. `transformFiles` is a niche pre-processing hook.

### 5.2 Anatomy

| Part              | ars-ui              | Ark UI                             | Notes                           |
| ----------------- | ------------------- | ---------------------------------- | ------------------------------- |
| Root              | `Root`              | `Root`                             | Equivalent                      |
| Label             | `Label`             | `Label`                            | Equivalent                      |
| Dropzone          | `Dropzone`          | `Dropzone`                         | Equivalent                      |
| Trigger           | `Trigger`           | `Trigger`                          | Equivalent                      |
| ItemGroup         | `ItemGroup`         | `ItemGroup`                        | Equivalent                      |
| Item              | `Item`              | `Item`                             | Equivalent                      |
| ItemName          | `ItemName`          | `ItemName`                         | Equivalent                      |
| ItemSizeText      | `ItemSizeText`      | `ItemSizeText`                     | Equivalent                      |
| ItemDeleteTrigger | `ItemDeleteTrigger` | `ItemDeleteTrigger`                | Equivalent                      |
| ItemProgress      | `ItemProgress`      | --                                 | ars-ui has progress indicator   |
| HiddenInput       | `HiddenInput`       | `HiddenInput`                      | Equivalent                      |
| ClearTrigger      | --                  | `ClearTrigger`                     | Ark has a clear-all button part |
| ItemPreview/Image | --                  | `ItemPreview` / `ItemPreviewImage` | Ark has image preview parts     |

**Gaps:** None critical. `ClearTrigger` functionality exists via `clear_files()` API method. Image preview is an adapter rendering concern.

### 5.3 Events

| Callback      | ars-ui                    | Ark UI         | Notes                               |
| ------------- | ------------------------- | -------------- | ----------------------------------- |
| File accepted | `Bindable` reactivity     | `onFileAccept` | Equivalent via binding              |
| File changed  | `Bindable` reactivity     | `onFileChange` | Equivalent via binding              |
| File rejected | rejection list on context | `onFileReject` | ars-ui stores rejections in context |

**Gaps:** None.

### 5.4 Features

| Feature                           | ars-ui | Ark UI                     |
| --------------------------------- | ------ | -------------------------- |
| Drag and drop                     | Yes    | Yes                        |
| File validation (type/size/count) | Yes    | Yes                        |
| Upload progress tracking          | Yes    | No (Ark is selection-only) |
| Retry failed uploads              | Yes    | No                         |
| Cancel uploads                    | Yes    | No                         |
| Directory upload                  | Yes    | Yes                        |
| Camera capture                    | Yes    | Yes                        |
| Read-only mode                    | Yes    | Yes                        |
| Auto-upload                       | Yes    | No                         |

**Gaps:** None. ars-ui exceeds Ark UI in upload lifecycle features.

### 5.5 Summary

- **Overall:** Full parity, with additional upload lifecycle features.
- **Divergences:** Ark UI's FileUpload is selection-only (no upload tracking). ars-ui adds full upload lifecycle (progress, retry, cancel). Ark has `preventDocumentDrop` as a convenience prop; ars-ui leaves this to the adapter.
- **Recommended additions:** None.
