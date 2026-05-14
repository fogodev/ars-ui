use ars_leptos::prelude::{TabKey, Translate};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en-US")]
pub(crate) enum CategoryTab {
    #[translate(en_US = "Input", pt_BR = "Entrada")]
    Input,

    #[translate(en_US = "Selection", pt_BR = "Seleção")]
    Selection,

    #[translate(en_US = "Overlay", pt_BR = "Sobreposição")]
    Overlay,

    #[translate(en_US = "Navigation", pt_BR = "Navegação")]
    Navigation,

    #[translate(en_US = "Date & time", pt_BR = "Data e hora")]
    DateTime,

    #[translate(en_US = "Data display", pt_BR = "Exibição de dados")]
    DataDisplay,

    #[translate(en_US = "Layout", pt_BR = "Layout")]
    Layout,

    #[translate(en_US = "Specialized", pt_BR = "Especializados")]
    Specialized,

    #[translate(en_US = "Utility", pt_BR = "Utilitários")]
    Utility,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en-US")]
pub(crate) enum NavigationTab {
    #[translate(en_US = "Overview", pt_BR = "Visão geral")]
    Overview,

    #[translate(en_US = "Keyboard", pt_BR = "Teclado")]
    Keyboard,

    #[translate(en_US = "Closable", pt_BR = "Fechável")]
    Closable,

    #[translate(en_US = "Disabled", pt_BR = "Desabilitada")]
    Disabled,
}

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum WidgetsText {
    #[translate(en_US = "Tailwind styling", pt_BR = "Estilo Tailwind")]
    TailwindStyling,

    #[translate(
        en_US = "Leptos Component Gallery",
        pt_BR = "Galeria de Componentes Leptos"
    )]
    LeptosTitle,

    #[translate(
        en_US = "One tab per spec category. Browse implemented components, or peek at categories that are still empty.",
        pt_BR = "Uma aba por categoria da especificação. Navegue pelos componentes implementados ou veja categorias que ainda estão vazias."
    )]
    PageSummary,

    #[translate(en_US = "Locale", pt_BR = "Idioma")]
    LocaleLabel,

    #[translate(en_US = "en-US", pt_BR = "en-US")]
    LocaleEnglish,

    #[translate(en_US = "pt-BR", pt_BR = "pt-BR")]
    LocalePortuguese,

    #[translate(
        en_US = "Input components - text-field, checkbox, slider, number-input, etc. Coming soon.",
        pt_BR = "Componentes de entrada - campo de texto, checkbox, slider, entrada numérica etc. Em breve."
    )]
    InputPanel,

    #[translate(
        en_US = "Selection components - select, combobox, listbox, menu, tags-input, etc. Coming soon.",
        pt_BR = "Componentes de seleção - select, combobox, listbox, menu, tags-input etc. Em breve."
    )]
    SelectionPanel,

    #[translate(
        en_US = "Overlay components - dialog, popover, tooltip, toast, presence, etc. Coming soon.",
        pt_BR = "Componentes de sobreposição - dialog, popover, tooltip, toast, presence etc. Em breve."
    )]
    OverlayPanel,

    #[translate(
        en_US = "Date and time components - date-field, time-field, calendar, date-picker, etc. Coming soon.",
        pt_BR = "Componentes de data e hora - date-field, time-field, calendar, date-picker etc. Em breve."
    )]
    DateTimePanel,

    #[translate(
        en_US = "Data display components - table, avatar, progress, meter, badge, etc. Coming soon.",
        pt_BR = "Componentes de exibição de dados - table, avatar, progress, meter, badge etc. Em breve."
    )]
    DataDisplayPanel,

    #[translate(
        en_US = "Layout components - splitter, scroll-area, carousel, portal, toolbar, etc. Coming soon.",
        pt_BR = "Componentes de layout - splitter, scroll-area, carousel, portal, toolbar etc. Em breve."
    )]
    LayoutPanel,

    #[translate(
        en_US = "Specialized components - color-picker, file-upload, signature-pad, qr-code, etc. Coming soon.",
        pt_BR = "Componentes especializados - color-picker, file-upload, signature-pad, qr-code etc. Em breve."
    )]
    SpecializedPanel,

    #[translate(en_US = "Tabs", pt_BR = "Abas")]
    TabsHeading,

    #[translate(
        en_US = "Live demo of the Tabs adapter - drag tabs to reorder, close the removable tabs, and inspect the disabled state.",
        pt_BR = "Demo ao vivo do adaptador de abas - arraste abas para reordenar, feche as abas removíveis e inspecione o estado desabilitado."
    )]
    TabsDemoSummary,

    #[translate(
        en_US = "Tabs is the first navigation primitive shipped in this gallery. The category tabs above use the same component.",
        pt_BR = "Abas são o primeiro primitivo de navegação nesta galeria. As abas de categoria acima usam o mesmo componente."
    )]
    TabsOverview,

    #[translate(
        en_US = "Arrow keys move focus across tabs (loop_focus on by default).",
        pt_BR = "As setas movem o foco entre as abas (loop_focus fica ativo por padrão)."
    )]
    KeyboardArrowKeys,

    #[translate(
        en_US = "Home / End jump to the first / last enabled tab.",
        pt_BR = "Home / End pulam para a primeira / última aba habilitada."
    )]
    KeyboardHomeEnd,

    #[translate(
        en_US = "In manual activation mode, Enter / Space activates the focused tab.",
        pt_BR = "No modo de ativação manual, Enter / Espaço ativa a aba focada."
    )]
    KeyboardManualActivation,

    #[translate(
        en_US = "Drag tabs to reorder them, or use Ctrl + Arrow keys.",
        pt_BR = "Arraste abas para reordená-las ou use Ctrl + setas."
    )]
    KeyboardReorder,

    #[translate(
        en_US = "This tab is closable, so Delete / Backspace removes it too.",
        pt_BR = "Esta aba é fechável, então Delete / Backspace também a remove."
    )]
    KeyboardClosable,

    #[translate(
        en_US = "Closable tabs render an extra close button and accept Delete / Backspace to fire CloseTab.",
        pt_BR = "Abas fecháveis renderizam um botão extra de fechar e aceitam Delete / Backspace para disparar CloseTab."
    )]
    ClosablePanel,

    #[translate(
        en_US = "Disabled tabs stay in the DOM for layout parity but are skipped by selection, keyboard focus, and drag reorder.",
        pt_BR = "Abas desabilitadas permanecem no DOM para paridade de layout, mas são ignoradas por seleção, foco por teclado e reordenação por arraste."
    )]
    DisabledPanel,

    #[translate(en_US = "Button variants", pt_BR = "Variantes de botão")]
    ButtonVariants,

    #[translate(
        en_US = "Hover each button to inspect transitions.",
        pt_BR = "Passe o mouse em cada botão para inspecionar as transições."
    )]
    ButtonVariantsNote,

    #[translate(en_US = "Default", pt_BR = "Padrão")]
    DefaultButton,

    #[translate(en_US = "Primary", pt_BR = "Primário")]
    PrimaryButton,

    #[translate(en_US = "Secondary", pt_BR = "Secundário")]
    SecondaryButton,

    #[translate(en_US = "Destructive", pt_BR = "Destrutivo")]
    DestructiveButton,

    #[translate(en_US = "Outline", pt_BR = "Contorno")]
    OutlineButton,

    #[translate(en_US = "Ghost", pt_BR = "Fantasma")]
    GhostButton,

    #[translate(en_US = "Link", pt_BR = "Link")]
    LinkButton,

    #[translate(en_US = "Button sizes", pt_BR = "Tamanhos de botão")]
    ButtonSizes,

    #[translate(en_US = "Small", pt_BR = "Pequeno")]
    SmallButton,

    #[translate(en_US = "Medium", pt_BR = "Médio")]
    MediumButton,

    #[translate(en_US = "Large", pt_BR = "Grande")]
    LargeButton,

    #[translate(en_US = "R", pt_BR = "R")]
    IconButton,

    #[translate(en_US = "Button states", pt_BR = "Estados de botão")]
    ButtonStates,

    #[translate(en_US = "Disabled", pt_BR = "Desabilitado")]
    DisabledButton,

    #[translate(en_US = "Loading", pt_BR = "Carregando")]
    LoadingButton,

    #[translate(en_US = "As child", pt_BR = "Como filho")]
    AsChild,

    #[translate(en_US = "Docs link root", pt_BR = "Link de docs como raiz")]
    DocsLinkRoot,

    #[translate(en_US = "Anchor as primary", pt_BR = "Âncora como primário")]
    AnchorAsPrimary,

    #[translate(en_US = "Forms", pt_BR = "Formulários")]
    Forms,

    #[translate(en_US = "Submit override", pt_BR = "Sobrescrever envio")]
    SubmitOverride,

    #[translate(en_US = "Reset", pt_BR = "Redefinir")]
    Reset,

    #[translate(en_US = "Visually hidden", pt_BR = "Visualmente oculto")]
    VisuallyHidden,

    #[translate(
        en_US = "Screen-reader text stays in the DOM while the visual layout remains quiet.",
        pt_BR = "O texto para leitores de tela permanece no DOM enquanto o leiaute visual fica limpo."
    )]
    VisuallyHiddenDescription,

    #[translate(
        en_US = "Screen reader only label",
        pt_BR = "Rótulo apenas para leitor de tela"
    )]
    VisuallyHiddenLabel,

    #[translate(en_US = "Skip to button variants", pt_BR = "Pular para variantes de botão")]
    FocusableSkipLink,

    #[translate(
        en_US = "Hidden label on consumer root",
        pt_BR = "Rótulo oculto na raiz do consumidor"
    )]
    AsChildHiddenLabel,

    #[translate(en_US = "Separator", pt_BR = "Separador")]
    SeparatorPrimitive,

    #[translate(
        en_US = "Semantic, vertical, and decorative separators share the same root part.",
        pt_BR = "Separadores semânticos, verticais e decorativos compartilham a mesma parte raiz."
    )]
    SeparatorDescription,

    #[translate(en_US = "Horizontal section break", pt_BR = "Quebra horizontal de seção")]
    HorizontalSeparator,

    #[translate(en_US = "Vertical divider", pt_BR = "Divisor vertical")]
    VerticalSeparator,

    #[translate(en_US = "Decorative divider", pt_BR = "Divisor decorativo")]
    DecorativeSeparator,

    #[translate(
        en_US = "Consumer-owned divider keeps separator semantics",
        pt_BR = "O divisor da raiz do consumidor preserva a semântica de separador"
    )]
    AsChildSeparator,

    #[translate(en_US = "Dismissable primitive", pt_BR = "Primitivo dismissable")]
    DismissablePrimitive,

    #[translate(en_US = "sm, md, lg, icon", pt_BR = "sm, md, lg, ícone")]
    ButtonSizeTokens,

    #[translate(
        en_US = "Disabled and busy controls.",
        pt_BR = "Controles desabilitados e ocupados."
    )]
    ButtonStatesNote,

    #[translate(
        en_US = "Button attrs on consumer-owned anchors.",
        pt_BR = "Atributos de botão em âncoras controladas pelo consumidor."
    )]
    AsChildNote,

    #[translate(
        en_US = "Submit/reset and form overrides.",
        pt_BR = "Envio, redefinição e sobrescritas de formulário."
    )]
    FormsNote,

    #[translate(
        en_US = "Outside pointer/focus, Escape, and hidden dismiss buttons share one primitive.",
        pt_BR = "Ponteiro/foco externo, Escape e botões ocultos de dispensar compartilham um primitivo."
    )]
    DismissableNote,

    #[translate(
        en_US = "Tailwind dismissable region",
        pt_BR = "Região dismissable em Tailwind"
    )]
    TailwindDismissableRegion,

    #[translate(
        en_US = "This standalone primitive is the behavior layer future overlays will compose.",
        pt_BR = "Este primitivo independente e a camada de comportamento que futuras sobreposições vão compor."
    )]
    DismissableCompositionDescription,

    #[translate(
        en_US = "Healthy and captured child output.",
        pt_BR = "Saída de filho saudável e capturada."
    )]
    ErrorBoundaryNote,

    #[translate(
        en_US = "Click outside the region, press Escape, or tab to a hidden dismiss button.",
        pt_BR = "Clique fora da região, pressione Escape ou use Tab até um botão oculto de dispensar."
    )]
    DismissInitial,

    #[translate(
        en_US = "Last dismiss reason: {reason}",
        pt_BR = "Último motivo de dispensa: {reason}"
    )]
    DismissReason { reason: String },

    #[translate(
        en_US = "Example child failed while rendering.",
        pt_BR = "O filho de exemplo falhou durante a renderização."
    )]
    ExampleChildError,

    #[translate(en_US = "Error boundary", pt_BR = "Limite de erro")]
    ErrorBoundary,

    #[translate(
        en_US = "Healthy child rendered inside the boundary.",
        pt_BR = "Filho saudável renderizado dentro do limite."
    )]
    HealthyChild,
}
