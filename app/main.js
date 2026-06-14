const { invoke, convertFileSrc } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;
const { open, save } = window.__TAURI__.dialog;

const elements = {
  windowTitlebar: document.querySelector("#window-titlebar"),
  windowFrame: document.querySelector("#window-frame"),
  windowDragRegion: document.querySelector("#window-drag-region"),
  windowTitlebarSubtitle: document.querySelector("#window-titlebar-subtitle"),
  fileMenuButton: document.querySelector("#file-menu-button"),
  fileMenu: document.querySelector("#file-menu"),
  fileMenuSave: document.querySelector("#file-menu-save"),
  fileMenuSaveAs: document.querySelector("#file-menu-save-as"),
  appearanceToggle: document.querySelector("#appearance-toggle"),
  windowMinimize: document.querySelector("#window-minimize"),
  windowMaximize: document.querySelector("#window-maximize"),
  windowClose: document.querySelector("#window-close"),
  openButtons: [
    document.querySelector("#open-pdf"),
    document.querySelector("#open-pdf-secondary"),
  ],
  appearancePanel: document.querySelector("#appearance-panel"),
  appearanceResetColumns: document.querySelector("#appearance-reset-columns"),
  appearanceReset: document.querySelector("#appearance-reset"),
  appearanceTheme: document.querySelector("#appearance-theme"),
  uiFontFamily: document.querySelector("#ui-font-family"),
  uiFontSize: document.querySelector("#ui-font-size"),
  monoFontFamily: document.querySelector("#mono-font-family"),
  monoFontSize: document.querySelector("#mono-font-size"),
  treeColumnResizers: Array.from(document.querySelectorAll(".column-resizer")),
  workspaceTabs: document.querySelector("#workspace-tabs"),
  workspaceTabOverflowButton: document.querySelector("#workspace-tab-overflow-button"),
  workspaceTabOverflowMenu: document.querySelector("#workspace-tab-overflow-menu"),
  workspaceTabPanels: Array.from(document.querySelectorAll("[data-tab-panel]")),
  navTabButtons: Array.from(document.querySelectorAll(".nav-item[data-workspace-tab]")),
  dropZone: document.querySelector("#drop-zone"),
  recentFilesList: document.querySelector("#recent-files-list"),
  clearRecentFiles: document.querySelector("#clear-recent-files"),
  errorPanel: document.querySelector("#error-panel"),
  errorPanelTitle: document.querySelector("#error-panel-title"),
  errorMessage: document.querySelector("#error-message"),
  documentLoading: document.querySelector("#document-loading"),
  documentLoadingMessage: document.querySelector("#document-loading-message"),
  loadState: document.querySelector("#load-state"),
  fileName: document.querySelector("#file-name"),
  fileSize: document.querySelector("#file-size"),
  pdfVersion: document.querySelector("#pdf-version"),
  pageCount: document.querySelector("#page-count"),
  objectCount: document.querySelector("#object-count"),
  streamCount: document.querySelector("#stream-count"),
  xrefCount: document.querySelector("#xref-count"),
  encrypted: document.querySelector("#encrypted"),
  parseWarningCount: document.querySelector("#parse-warning-count"),
  documentOpenStatus: document.querySelector("#document-open-status"),
  pageListStatus: document.querySelector("#page-list-status"),
  overviewPreviewStatus: document.querySelector("#overview-preview-status"),
  pageListCount: document.querySelector("#page-list-count"),
  pageList: document.querySelector("#page-list"),
  pageMetadataPanel: document.querySelector("#page-metadata-panel"),
  pageMetadataPlaceholder: document.querySelector("#page-metadata-placeholder"),
  pageMetadataFloat: document.querySelector("#page-metadata-float"),
  pageMetadataSubtitle: document.querySelector("#page-metadata-subtitle"),
  pageMetadataState: document.querySelector("#page-metadata-state"),
  pagePreviewState: document.querySelector("#page-preview-state"),
  pagePreviewStatus: document.querySelector("#page-preview-status"),
  pagePreviewDraftStatus: document.querySelector("#page-preview-draft-status"),
  pagePreviewDraftTitle: document.querySelector("#page-preview-draft-title"),
  pagePreviewDraftMessage: document.querySelector("#page-preview-draft-message"),
  saveEditsAndRerender: document.querySelector("#save-edits-and-rerender"),
  pagePreviewError: document.querySelector("#page-preview-error"),
  pagePreviewFigure: document.querySelector("#page-preview-figure"),
  pagePreviewViewport: document.querySelector("#page-preview-viewport"),
  pagePreviewEmpty: document.querySelector("#page-preview-empty"),
  pagePreviewEmptyTitle: document.querySelector("#page-preview-empty-title"),
  pagePreviewEmptyMessage: document.querySelector("#page-preview-empty-message"),
  pagePreviewCanvas: document.querySelector(".page-preview-canvas"),
  pagePreviewStage: document.querySelector("#page-preview-stage"),
  pagePreviewImage: document.querySelector("#page-preview-image"),
  pagePreviewCaption: document.querySelector("#page-preview-caption"),
  pageZoomOut: document.querySelector("#page-zoom-out"),
  pageZoomIn: document.querySelector("#page-zoom-in"),
  pageZoomReset: document.querySelector("#page-zoom-reset"),
  pageZoomLabel: document.querySelector("#page-zoom-label"),
  pageObjectOverlay: document.querySelector("#page-object-overlay"),
  pageObjectOverlayLabel: document.querySelector("#page-object-overlay-label"),
  pageReference: document.querySelector("#page-reference"),
  pageRotation: document.querySelector("#page-rotation"),
  pageMediaBox: document.querySelector("#page-mediabox"),
  pageCropBox: document.querySelector("#page-cropbox"),
  pageBleedBox: document.querySelector("#page-bleedbox"),
  pageTrimBox: document.querySelector("#page-trimbox"),
  pageArtBox: document.querySelector("#page-artbox"),
  pageResourceFonts: document.querySelector("#page-resource-fonts"),
  pageResourceXobjects: document.querySelector("#page-resource-xobjects"),
  pageResourceImages: document.querySelector("#page-resource-images"),
  pageResourceContents: document.querySelector("#page-resource-contents"),
  pageResourceAnnotations: document.querySelector("#page-resource-annotations"),
  pageLinksSection: document.querySelector("#page-links-section"),
  pageReferenceSearch: document.querySelector("#page-reference-search"),
  pageReferenceCount: document.querySelector("#page-reference-count"),
  pageObjectLinks: document.querySelector("#page-object-links"),
  pageObjectsStatus: document.querySelector("#page-objects-status"),
  pageObjectsList: document.querySelector("#page-objects-list"),
  pageObjectWarningsSection: document.querySelector("#page-object-warnings-section"),
  pageObjectWarnings: document.querySelector("#page-object-warnings"),
  pageSelectedObjectSection: document.querySelector("#page-selected-object-section"),
  pageSelectedObjectEmpty: document.querySelector("#page-selected-object-empty"),
  pageSelectedObjectTable: document.querySelector("#page-selected-object-table"),
  pageSelectedObjectProperties: document.querySelector("#page-selected-object-properties"),
  pageSelectedObjectNote: document.querySelector("#page-selected-object-note"),
  pageSelectedObjectOpen: document.querySelector("#page-selected-object-open"),
  objectTreeSearch: document.querySelector("#object-tree-search"),
  objectTreeSearchStatus: document.querySelector("#object-tree-search-status"),
  objectTree: document.querySelector("#object-tree"),
  treeCount: document.querySelector("#tree-count"),
  trailerSubtitle: document.querySelector("#trailer-subtitle"),
  trailerState: document.querySelector("#trailer-state"),
  trailerError: document.querySelector("#trailer-error"),
  trailerScroll: document.querySelector("#trailer-scroll"),
  trailerEntries: document.querySelector("#trailer-entries"),
  acroformSubtitle: document.querySelector("#acroform-subtitle"),
  acroformState: document.querySelector("#acroform-state"),
  acroformError: document.querySelector("#acroform-error"),
  acroformWarnings: document.querySelector("#acroform-warnings"),
  acroformSearch: document.querySelector("#acroform-search"),
  acroformCount: document.querySelector("#acroform-count"),
  acroformList: document.querySelector("#acroform-list"),
  annotsSubtitle: document.querySelector("#annots-subtitle"),
  annotsState: document.querySelector("#annots-state"),
  annotsError: document.querySelector("#annots-error"),
  annotsWarnings: document.querySelector("#annots-warnings"),
  annotsSearch: document.querySelector("#annots-search"),
  annotsCount: document.querySelector("#annots-count"),
  annotsList: document.querySelector("#annots-list"),
  inspectorSubtitle: document.querySelector("#inspector-subtitle"),
  inspectorState: document.querySelector("#inspector-state"),
  inspectorError: document.querySelector("#inspector-error"),
  inspectorReference: document.querySelector("#inspector-reference"),
  inspectorType: document.querySelector("#inspector-type"),
  inspectorSummary: document.querySelector("#inspector-summary"),
  inspectorRange: document.querySelector("#inspector-range"),
  inspectorRawLength: document.querySelector("#inspector-raw-length"),
  inspectorKeys: document.querySelector("#inspector-keys"),
  streamSection: document.querySelector("#stream-section"),
  streamDeclaredLength: document.querySelector("#stream-declared-length"),
  streamActualLength: document.querySelector("#stream-actual-length"),
  streamFilters: document.querySelector("#stream-filters"),
  streamDecodedLength: document.querySelector("#stream-decoded-length"),
  streamDecodeIssues: document.querySelector("#stream-decode-issues"),
  openStreamDetails: document.querySelector("#open-stream-details"),
  objectDetailsScroll: document.querySelector("#object-details-scroll"),
  objectDetailsEntries: document.querySelector("#object-details-entries"),
  objectEditStatus: document.querySelector("#object-edit-status"),
  revertObjectEdits: document.querySelector("#revert-object-edits"),
  revertAllEdits: document.querySelector("#revert-all-edits"),
  saveModifiedPdf: document.querySelector("#save-modified-pdf"),
  streamViewerSubtitle: document.querySelector("#stream-viewer-subtitle"),
  streamViewerState: document.querySelector("#stream-viewer-state"),
  streamViewerError: document.querySelector("#stream-viewer-error"),
  streamViewerStatus: document.querySelector("#stream-viewer-status"),
  streamViewerReference: document.querySelector("#stream-viewer-reference"),
  streamViewerRawLength: document.querySelector("#stream-viewer-raw-length"),
  streamViewerByteRange: document.querySelector("#stream-viewer-byte-range"),
  streamViewerDecodedLength: document.querySelector("#stream-viewer-decoded-length"),
  streamViewerFilters: document.querySelector("#stream-viewer-filters"),
  streamViewerIssues: document.querySelector("#stream-viewer-issues"),
  streamModeButtons: Array.from(document.querySelectorAll(".stream-mode-button")),
  copyDecodedStream: document.querySelector("#copy-decoded-stream"),
  exportDecodedStream: document.querySelector("#export-decoded-stream"),
  editDecodedStream: document.querySelector("#edit-decoded-stream"),
  renderStreamImage: document.querySelector("#render-stream-image"),
  streamEditPanel: document.querySelector("#stream-edit-panel"),
  streamEditHint: document.querySelector("#stream-edit-hint"),
  streamEditTextarea: document.querySelector("#stream-edit-textarea"),
  applyStreamEdit: document.querySelector("#apply-stream-edit"),
  cancelStreamEdit: document.querySelector("#cancel-stream-edit"),
  streamImagePreview: document.querySelector("#stream-image-preview"),
  streamImagePreviewTitle: document.querySelector("#stream-image-preview-title"),
  streamImagePreviewMeta: document.querySelector("#stream-image-preview-meta"),
  streamImagePreviewImage: document.querySelector("#stream-image-preview-image"),
  closeStreamImagePreview: document.querySelector("#close-stream-image-preview"),
  streamViewerContent: document.querySelector("#stream-viewer-content"),
};

let currentPdfPath = null;
let selectedTreeButton = null;
let treeButtonsByReference = new Map();
let navigationHistory = [];
let navigationIndex = -1;
let activeReference = null;
let navigationRequestId = 0;
let activeStreamView = null;
let activeStreamMode = "hex";
let streamViewRequestId = 0;
let streamImagePreviewRequestId = 0;
let contentAnalysisRequestId = 0;
let activeStreamReference = null;
let inspectedStreamReference = null;
let activeContentTokens = [];
let activeContentOperators = [];
let streamVirtualTextKey = null;
let activePages = [];
let selectedPageButton = null;
let activePage = null;
let activePageObjectInspection = null;
let activePageObjects = [];
let activePageLinks = [];
let pageReferenceSearchQuery = "";
let selectedPageObject = null;
let selectedPageObjectRow = null;
let isPageMetadataFloating = false;
let pageMetadataFloatFrame = null;
let pagePreviewRequestId = 0;
let pageObjectsRequestId = 0;
let pageSelectionWorkTimer = null;
let pageDetailsInFlight = false;
let pendingPageDetailsLoad = null;
let pageObjectsLoadTimer = null;
let pageObjectsInFlight = false;
let pendingPageObjectsLoad = null;
let pagePreviewZoom = 1;
let pagePreviewRenderTimer = null;
let pagePreviewRenderZoom = null;
let pagePreviewRenderAnchor = null;
let pagePreviewRenderInFlight = false;
let pendingPagePreviewRender = null;
let pagePreviewZoomFrame = null;
let pagePreviewSwapFrame = null;
let pendingPagePreviewZoomAnchor = null;
let lastPagePreviewZoomAnchor = null;
let activePagePreview = null;
let workspaceTabs = [];
let activeWorkspaceTabId = "overview";
let workspaceTabOverflowOpen = false;
let isPdfLoading = false;
let pageListRequestId = 0;
let pageDetailsRequestId = 0;
let pdfOpenGeneration = 0;
let currentOpenMode = "none";
let activeTrailer = null;
let activeAcroForm = null;
let acroFormRequestId = 0;
let acroFormSearchQuery = "";
let activeAnnotations = null;
let annotsSearchQuery = "";
let trailerExpandedKeys = new Set();
let trailerVisibleChildCounts = new Map();
let trailerLoadedObjects = new Map();
let trailerLoadingObjects = new Set();
let trailerLoadErrors = new Map();
let activeObjectDetailsNode = null;
let activeObjectInspection = null;
let objectDetailsExpandedKeys = new Set();
let objectDetailsVisibleChildCounts = new Map();
let objectDetailsLoadedObjects = new Map();
let objectDetailsLoadingObjects = new Set();
let objectDetailsLoadErrors = new Map();
let objectTreeVisibleChildCounts = new Map();
let objectTreeExpandedKeys = new Set();
let activeObjectTree = null;
let objectTreeSearchQuery = "";
let objectTreeSearchRows = [];
let virtualPageListState = null;
let virtualObjectTreeState = null;
let virtualTrailerTreeState = null;
let virtualObjectDetailsTreeState = null;
let virtualStreamTextState = null;
let objectInspectionCache = new Map();
let objectInspectionLoading = new Map();
let streamMetadataCache = new Map();
let streamMetadataLoading = new Map();
let streamViewCache = new Map();
let streamViewLoading = new Map();
let streamPreviewCache = new Map();
let streamPreviewLoading = new Map();
let streamImagePreviewCache = new Map();
let streamImagePreviewLoading = new Map();
let pagePreviewCache = new Map();
let pagePreviewLoading = new Map();
let acroFormCache = new Map();
let acroFormLoading = new Map();
let lastUserInteractionAt = 0;
let recentFiles = [];
let appearanceSettings = null;
let appearancePanelOpen = false;
let fileMenuOpen = false;
let treeColumnWidths = null;
let activeTreeColumnResize = null;
let objectEditDrafts = new Map();
let activeObjectEditKey = null;
let activeObjectEditPathKey = null;
let streamEditDrafts = new Map();
let activeStreamEditKey = null;
let objectEditDraftRevision = 0;
let draftPreviewSnapshots = new Map();
let draftPreviewSnapshotLoading = new Map();
const TRAILER_TREE_MAX_DEPTH = 12;
const TRAILER_TREE_MAX_ROWS = 800;
const TRAILER_TREE_CHILD_BATCH = 120;
const OBJECT_TREE_CHILD_BATCH = 160;
const OBJECT_TREE_RENDER_BUDGET = 1400;
const VIRTUALIZATION_THRESHOLD = 120;
const VIRTUALIZATION_OVERSCAN = 8;
const PAGE_LIST_ROW_HEIGHT = 82;
const OBJECT_TREE_ROW_HEIGHT = 30;
const TRAILER_TREE_ROW_HEIGHT = 34;
const TASK_PRIORITY_USER = 100;
const TASK_PRIORITY_PREFETCH = 10;
const TASK_KIND_LIMITS = {
  pageMetadata: 2,
  pagePreview: 1,
  pageObjects: 1,
  objectInspect: 2,
  trailerObject: 2,
  streamMetadata: 2,
  streamPreview: 1,
  prefetchPageMetadata: 1,
};
const singletonTabDefinitions = {
  overview: { id: "overview", kind: "singleton", panel: "overview", title: "Overview", closeable: false },
  object: { id: "object", kind: "singleton", panel: "object", title: "Inspector", closeable: true },
  trailer: { id: "trailer", kind: "singleton", panel: "trailer", title: "Trailer", closeable: true },
  annots: { id: "annots", kind: "singleton", panel: "annots", title: "Annots", closeable: true },
  acroform: { id: "acroform", kind: "singleton", panel: "acroform", title: "AcroForm", closeable: true },
  stream: { id: "stream", kind: "singleton", panel: "stream", title: "Stream Viewer", closeable: true },
  page: { id: "page", kind: "singleton", panel: "page", title: "Pages", closeable: true },
};
const PAGE_PREVIEW_MIN_ZOOM = 0.5;
const PAGE_PREVIEW_MAX_ZOOM = 4;
const PAGE_PREVIEW_ZOOM_STEP = 0.25;
const PAGE_PREVIEW_WHEEL_ZOOM_STEP = 0.25;
const PAGE_PREVIEW_RENDER_DEBOUNCE_MS = 300;
const PAGE_SELECTION_SETTLE_MS = 90;
const PAGE_OBJECTS_LOAD_DEBOUNCE_MS = 120;
const PREFETCH_INTERACTION_PAUSE_MS = 1500;
const STREAM_TEXT_RENDER_LIMIT = 64 * 1024;
const STREAM_VIRTUAL_ROW_HEIGHT = 18;
const STREAM_VIRTUAL_OVERSCAN = 12;
const STREAM_VIRTUAL_MAX_ROW_CHARS = 2048;
const STREAM_RAW_BINARY_RENDER_LIMIT = 8 * 1024;
const STREAM_DECODED_PENDING_MESSAGE = "Decoded preview has not loaded yet.";
const RECENT_FILES_STORAGE_KEY = "pdf-debugger.recent-files";
const RECENT_FILES_LIMIT = 8;
const APPEARANCE_STORAGE_KEY = "pdf-debugger.appearance";
const TREE_COLUMN_STORAGE_KEY = "pdf-debugger.tree-column-widths";
const DEFAULT_APPEARANCE_SETTINGS = {
  theme: "system",
  uiFontFamily: "system",
  uiFontSize: 14,
  monoFontFamily: "system",
  monoFontSize: 12,
};
const APPEARANCE_THEMES = new Set(["system", "light", "dark"]);
const DEFAULT_TREE_COLUMN_WIDTHS = {
  key: 170,
  type: 180,
  value: 520,
};
const MIN_TREE_COLUMN_WIDTHS = {
  key: 120,
  type: 120,
  value: 220,
};
const MAX_TREE_COLUMN_WIDTH = 2400;
const UI_FONT_FAMILIES = {
  system: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  segoe: '"Segoe UI", Inter, ui-sans-serif, system-ui, sans-serif',
  yahei: '"Microsoft YaHei", "Segoe UI", Inter, ui-sans-serif, system-ui, sans-serif',
  simsun: 'SimSun, "Microsoft YaHei", "Segoe UI", serif',
  arial: 'Arial, "Segoe UI", ui-sans-serif, system-ui, sans-serif',
  inter: 'Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
};
const MONO_FONT_FAMILIES = {
  system: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
  consolas: 'Consolas, "Cascadia Mono", ui-monospace, monospace',
  cascadia: '"Cascadia Mono", Consolas, ui-monospace, monospace',
  jetbrains: '"JetBrains Mono", "Cascadia Mono", Consolas, ui-monospace, monospace',
  menlo: 'Menlo, Consolas, ui-monospace, monospace',
  monospace: 'monospace',
};

function formatBytes(bytes) {
  if (!Number.isFinite(bytes)) {
    return "-";
  }

  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const precision = unitIndex === 0 ? 0 : 1;
  return `${value.toFixed(precision)} ${units[unitIndex]}`;
}

function clampInteger(value, min, max, fallback) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return fallback;
  }
  return Math.max(min, Math.min(max, Math.round(number)));
}

function normalizeAppearanceSettings(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    theme: APPEARANCE_THEMES.has(source.theme) ? source.theme : DEFAULT_APPEARANCE_SETTINGS.theme,
    uiFontFamily: UI_FONT_FAMILIES[source.uiFontFamily] ? source.uiFontFamily : DEFAULT_APPEARANCE_SETTINGS.uiFontFamily,
    uiFontSize: clampInteger(source.uiFontSize, 12, 18, DEFAULT_APPEARANCE_SETTINGS.uiFontSize),
    monoFontFamily: MONO_FONT_FAMILIES[source.monoFontFamily]
      ? source.monoFontFamily
      : DEFAULT_APPEARANCE_SETTINGS.monoFontFamily,
    monoFontSize: clampInteger(source.monoFontSize, 11, 16, DEFAULT_APPEARANCE_SETTINGS.monoFontSize),
  };
}

function resolveAppearanceTheme(theme) {
  if (theme === "dark" || theme === "light") {
    return theme;
  }
  return window.matchMedia?.("(prefers-color-scheme: dark)")?.matches ? "dark" : "light";
}

function loadAppearanceSettings() {
  try {
    const raw = window.localStorage.getItem(APPEARANCE_STORAGE_KEY);
    appearanceSettings = normalizeAppearanceSettings(raw ? JSON.parse(raw) : DEFAULT_APPEARANCE_SETTINGS);
  } catch (error) {
    appearanceSettings = normalizeAppearanceSettings(DEFAULT_APPEARANCE_SETTINGS);
  }
  applyAppearanceSettings();
  syncAppearanceControls();
}

function saveAppearanceSettings() {
  try {
    window.localStorage.setItem(APPEARANCE_STORAGE_KEY, JSON.stringify(appearanceSettings));
  } catch (error) {
    showRecoverableNotice(`Display settings could not be saved locally: ${String(error)}`);
  }
}

function applyAppearanceSettings() {
  const settings = normalizeAppearanceSettings(appearanceSettings);
  appearanceSettings = settings;
  const root = document.documentElement;
  root.dataset.themePreference = settings.theme;
  root.dataset.theme = resolveAppearanceTheme(settings.theme);
  root.style.colorScheme = root.dataset.theme;
  root.style.setProperty("--ui-font-family", UI_FONT_FAMILIES[settings.uiFontFamily]);
  root.style.setProperty("--ui-font-size", `${settings.uiFontSize}px`);
  root.style.setProperty("--mono-font-family", MONO_FONT_FAMILIES[settings.monoFontFamily]);
  root.style.setProperty("--mono-font-size", `${settings.monoFontSize}px`);
  scheduleWorkspaceTabOverflowUpdate();
  if (virtualStreamTextState) {
    scheduleVirtualRender(virtualStreamTextState);
  }
}

function syncAppearanceControls() {
  if (!appearanceSettings) {
    return;
  }
  elements.appearanceTheme.value = appearanceSettings.theme;
  elements.uiFontFamily.value = appearanceSettings.uiFontFamily;
  elements.uiFontSize.value = String(appearanceSettings.uiFontSize);
  elements.monoFontFamily.value = appearanceSettings.monoFontFamily;
  elements.monoFontSize.value = String(appearanceSettings.monoFontSize);
}

function updateAppearanceSetting(key, value) {
  appearanceSettings = normalizeAppearanceSettings({
    ...appearanceSettings,
    [key]: value,
  });
  applyAppearanceSettings();
  syncAppearanceControls();
  saveAppearanceSettings();
}

function resetAppearanceSettings() {
  appearanceSettings = normalizeAppearanceSettings(DEFAULT_APPEARANCE_SETTINGS);
  try {
    window.localStorage.removeItem(APPEARANCE_STORAGE_KEY);
  } catch (error) {
    showRecoverableNotice(`Display settings could not be reset locally: ${String(error)}`);
  }
  applyAppearanceSettings();
  syncAppearanceControls();
}

function normalizeTreeColumnWidths(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    key: clampInteger(source.key, MIN_TREE_COLUMN_WIDTHS.key, MAX_TREE_COLUMN_WIDTH, DEFAULT_TREE_COLUMN_WIDTHS.key),
    type: clampInteger(source.type, MIN_TREE_COLUMN_WIDTHS.type, MAX_TREE_COLUMN_WIDTH, DEFAULT_TREE_COLUMN_WIDTHS.type),
    value: clampInteger(source.value, MIN_TREE_COLUMN_WIDTHS.value, MAX_TREE_COLUMN_WIDTH, DEFAULT_TREE_COLUMN_WIDTHS.value),
  };
}

function loadTreeColumnWidths() {
  try {
    const raw = window.localStorage.getItem(TREE_COLUMN_STORAGE_KEY);
    treeColumnWidths = normalizeTreeColumnWidths(raw ? JSON.parse(raw) : DEFAULT_TREE_COLUMN_WIDTHS);
  } catch (error) {
    treeColumnWidths = normalizeTreeColumnWidths(DEFAULT_TREE_COLUMN_WIDTHS);
  }
  applyTreeColumnWidths();
}

function applyTreeColumnWidths() {
  const widths = normalizeTreeColumnWidths(treeColumnWidths);
  treeColumnWidths = widths;
  const root = document.documentElement;
  root.style.setProperty("--tree-key-column-width", `${widths.key}px`);
  root.style.setProperty("--tree-type-column-width", `${widths.type}px`);
  root.style.setProperty("--tree-value-column-width", `${widths.value}px`);
}

function saveTreeColumnWidths() {
  try {
    window.localStorage.setItem(TREE_COLUMN_STORAGE_KEY, JSON.stringify(normalizeTreeColumnWidths(treeColumnWidths)));
  } catch (error) {
    showRecoverableNotice(`Tree column widths could not be saved locally: ${String(error)}`);
  }
}

function resetTreeColumnWidths() {
  treeColumnWidths = normalizeTreeColumnWidths(DEFAULT_TREE_COLUMN_WIDTHS);
  try {
    window.localStorage.removeItem(TREE_COLUMN_STORAGE_KEY);
  } catch (error) {
    showRecoverableNotice(`Tree column widths could not be reset locally: ${String(error)}`);
  }
  applyTreeColumnWidths();
}

function startTreeColumnResize(event) {
  const column = event.currentTarget?.dataset?.resizeColumn;
  if (!column || !Object.prototype.hasOwnProperty.call(DEFAULT_TREE_COLUMN_WIDTHS, column)) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  if (!treeColumnWidths) {
    treeColumnWidths = normalizeTreeColumnWidths(DEFAULT_TREE_COLUMN_WIDTHS);
  }
  finishTreeColumnResize();
  activeTreeColumnResize = {
    column,
    handle: event.currentTarget,
    pointerId: event.pointerId,
    startX: event.clientX,
    startWidth: normalizeTreeColumnWidths(treeColumnWidths)[column],
  };
  activeTreeColumnResize.handle.classList.add("is-active");
  document.body.classList.add("is-resizing-tree-columns");
  try {
    activeTreeColumnResize.handle.setPointerCapture(event.pointerId);
  } catch (error) {
    // Some embedded WebViews may not allow pointer capture after synthetic events.
  }
  document.addEventListener("pointermove", updateTreeColumnResize);
  document.addEventListener("pointerup", finishTreeColumnResize);
  document.addEventListener("pointercancel", finishTreeColumnResize);
}

function updateTreeColumnResize(event) {
  if (!activeTreeColumnResize || event.pointerId !== activeTreeColumnResize.pointerId) {
    return;
  }
  event.preventDefault();
  const { column, startX, startWidth } = activeTreeColumnResize;
  const nextWidth = clampInteger(
    startWidth + event.clientX - startX,
    MIN_TREE_COLUMN_WIDTHS[column],
    MAX_TREE_COLUMN_WIDTH,
    startWidth
  );
  treeColumnWidths = normalizeTreeColumnWidths({
    ...treeColumnWidths,
    [column]: nextWidth,
  });
  applyTreeColumnWidths();
}

function finishTreeColumnResize(event) {
  if (!activeTreeColumnResize) {
    return;
  }
  if (event && event.pointerId !== activeTreeColumnResize.pointerId) {
    return;
  }
  const { handle, pointerId } = activeTreeColumnResize;
  handle.classList.remove("is-active");
  try {
    handle.releasePointerCapture(pointerId);
  } catch (error) {
    // Pointer capture may already be released if the pointer left the WebView.
  }
  activeTreeColumnResize = null;
  document.body.classList.remove("is-resizing-tree-columns");
  document.removeEventListener("pointermove", updateTreeColumnResize);
  document.removeEventListener("pointerup", finishTreeColumnResize);
  document.removeEventListener("pointercancel", finishTreeColumnResize);
  saveTreeColumnWidths();
}

function setAppearancePanelOpen(isOpen) {
  appearancePanelOpen = Boolean(isOpen);
  elements.appearancePanel.hidden = !appearancePanelOpen;
  elements.appearanceToggle?.setAttribute("aria-expanded", String(appearancePanelOpen));
}

function setFileMenuOpen(isOpen) {
  fileMenuOpen = Boolean(isOpen);
  if (elements.fileMenu) {
    elements.fileMenu.hidden = !fileMenuOpen;
  }
  if (elements.fileMenuButton) {
    elements.fileMenuButton.setAttribute("aria-expanded", String(fileMenuOpen));
  }
  if (fileMenuOpen) {
    updateFileMenuState();
  }
}

function updateFileMenuState() {
  const canSaveDraft = Boolean(currentPdfPath && hasDocumentDraftEdits());
  if (elements.fileMenuSave) {
    elements.fileMenuSave.disabled = !canSaveDraft;
  }
  if (elements.fileMenuSaveAs) {
    elements.fileMenuSaveAs.disabled = !canSaveDraft;
  }
}

function updateWindowSubtitle(path = currentPdfPath) {
  if (!elements.windowTitlebarSubtitle) {
    return;
  }
  elements.windowTitlebarSubtitle.textContent = "";
  elements.windowTitlebarSubtitle.removeAttribute("title");
}

async function minimizeWindow() {
  try {
    await getCurrentWindow().minimize();
  } catch (error) {
    showRecoverableNotice(`Window could not be minimized: ${String(error)}`);
  }
}

async function toggleWindowMaximize() {
  try {
    await getCurrentWindow().toggleMaximize();
    updateWindowMaximizeButton();
  } catch (error) {
    showRecoverableNotice(`Window could not be maximized or restored: ${String(error)}`);
  }
}

async function closeWindow() {
  try {
    await getCurrentWindow().close();
  } catch (error) {
    showRecoverableNotice(`Window could not be closed: ${String(error)}`);
  }
}

async function startWindowDrag(event) {
  if (event.target?.closest?.("[data-no-drag]")) {
    return;
  }
  if (event.button !== 0 || event.detail > 1) {
    return;
  }
  event.preventDefault();
  try {
    await getCurrentWindow().startDragging();
  } catch (error) {
    showRecoverableNotice(`Window could not start dragging: ${String(error)}`);
  }
}

async function updateWindowMaximizeButton() {
  if (!elements.windowMaximize) {
    return;
  }
  try {
    const maximized = await getCurrentWindow().isMaximized();
    elements.windowFrame?.classList.toggle("is-maximized", maximized);
    elements.windowMaximize.textContent = maximized ? "❐" : "□";
    elements.windowMaximize.title = maximized ? "Restore" : "Maximize";
    elements.windowMaximize.setAttribute("aria-label", maximized ? "Restore window" : "Maximize window");
  } catch (error) {
    elements.windowFrame?.classList.remove("is-maximized");
    elements.windowMaximize.textContent = "□";
  }
}

function loadRecentFiles() {
  try {
    const raw = window.localStorage.getItem(RECENT_FILES_STORAGE_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    recentFiles = Array.isArray(parsed)
      ? parsed
          .filter((item) => typeof item?.path === "string" && item.path.toLowerCase().endsWith(".pdf"))
          .map((item) => ({
            path: item.path,
            openedAt: Number(item.openedAt) || 0,
          }))
          .sort((left, right) => right.openedAt - left.openedAt)
          .slice(0, RECENT_FILES_LIMIT)
      : [];
  } catch (error) {
    recentFiles = [];
  }
  renderRecentFiles();
}

function saveRecentFiles() {
  try {
    window.localStorage.setItem(RECENT_FILES_STORAGE_KEY, JSON.stringify(recentFiles));
  } catch (error) {
    showRecoverableNotice(`Recent files could not be saved locally: ${String(error)}`);
  }
}

function rememberRecentFile(path) {
  if (typeof path !== "string" || !path.toLowerCase().endsWith(".pdf")) {
    return;
  }

  const normalizedPath = path.trim();
  recentFiles = [
    { path: normalizedPath, openedAt: Date.now() },
    ...recentFiles.filter((item) => item.path !== normalizedPath),
  ].slice(0, RECENT_FILES_LIMIT);
  saveRecentFiles();
  renderRecentFiles();
}

function clearRecentFiles() {
  recentFiles = [];
  try {
    window.localStorage.removeItem(RECENT_FILES_STORAGE_KEY);
  } catch (error) {
    showRecoverableNotice(`Recent files could not be cleared: ${String(error)}`);
  }
  renderRecentFiles();
}

function renderRecentFiles() {
  elements.recentFilesList.replaceChildren();
  elements.recentFilesList.classList.toggle("empty-list", recentFiles.length === 0);
  elements.clearRecentFiles.disabled = recentFiles.length === 0;
  if (!recentFiles.length) {
    elements.recentFilesList.textContent = "No recent PDFs yet.";
    return;
  }

  for (const item of recentFiles) {
    const button = document.createElement("button");
    button.className = "recent-file-row";
    button.type = "button";
    button.title = item.path;
    button.addEventListener("click", () => loadPdf(item.path));

    const main = document.createElement("span");
    main.className = "recent-file-main";
    const name = document.createElement("span");
    name.className = "recent-file-name";
    name.textContent = fileNameFromPath(item.path);
    main.appendChild(name);
    const path = document.createElement("span");
    path.className = "recent-file-path";
    path.textContent = item.path;
    main.appendChild(path);
    button.appendChild(main);

    const time = document.createElement("span");
    time.className = "recent-file-time";
    time.textContent = formatRecentFileTime(item.openedAt);
    button.appendChild(time);
    elements.recentFilesList.appendChild(button);
  }
}

function fileNameFromPath(path) {
  return String(path ?? "").split(/[\\/]/).filter(Boolean).pop() || "PDF";
}

function formatRecentFileTime(timestamp) {
  if (!Number.isFinite(timestamp) || timestamp <= 0) {
    return "Recent";
  }

  const elapsedMs = Date.now() - timestamp;
  const minute = 60 * 1000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (elapsedMs < minute) {
    return "Just now";
  }
  if (elapsedMs < hour) {
    return `${Math.max(1, Math.round(elapsedMs / minute))}m ago`;
  }
  if (elapsedMs < day) {
    return `${Math.max(1, Math.round(elapsedMs / hour))}h ago`;
  }
  if (elapsedMs < 7 * day) {
    return `${Math.max(1, Math.round(elapsedMs / day))}d ago`;
  }
  return new Date(timestamp).toLocaleDateString();
}

function objectLabel(reference) {
  return reference ? `${reference.object} ${reference.generation} R` : "";
}

function referenceKey(reference) {
  return reference ? `${reference.object}:${reference.generation}` : "";
}

function pageCacheKey(path, pageNumber) {
  return `${path}::page:${Number(pageNumber)}`;
}

function streamCacheKey(path, reference) {
  const normalized = normalizeReference(reference);
  return `${path}::stream:${referenceKey(normalized)}`;
}

function streamPreviewCacheKey(path, reference, mode) {
  return `${streamCacheKey(path, reference)}::preview:${mode}`;
}

function previewCacheKey(documentKey, pageNumber, zoom) {
  return `${documentKey}::preview:${Number(pageNumber)}:${Math.round(normalizedPreviewZoom(zoom) * 1000)}`;
}

function previewDocumentCacheKey(path, revision = 0) {
  return revision > 0 ? `${path}::draft:${revision}` : String(path ?? "");
}

function normalizeReference(reference) {
  return {
    object: Number(reference.object),
    generation: Number(reference.generation),
  };
}

function sameReference(left, right) {
  return referenceKey(left) === referenceKey(right);
}

function perfLogEnabled() {
  return window.localStorage.getItem("pdf-debugger.perf-log") === "1";
}

function perfMark(label, detail = "") {
  return {
    label,
    detail,
    startedAt: performance.now(),
  };
}

function perfDone(mark, status = "done") {
  if (!perfLogEnabled()) {
    return;
  }
  const detail = mark.detail ? ` ${mark.detail}` : "";
  console.debug(
    `[pdf-debugger perf] ${mark.label}${detail} status=${status} elapsed_ms=${Math.round(performance.now() - mark.startedAt)}`,
  );
}

function countObjectNodes(node) {
  if (!node) {
    return 0;
  }

  const current = node.object ? 1 : 0;
  return current + (node.children ?? []).reduce((sum, child) => sum + countObjectNodes(child), 0);
}

function childBatchKey(prefix, path) {
  return `${prefix}:${path}`;
}

function getChildBatchSize(map, key, defaultSize) {
  return map.get(key) ?? defaultSize;
}

function virtualRange(container, totalRows, rowHeight) {
  if (!container || totalRows <= 0 || totalRows < VIRTUALIZATION_THRESHOLD) {
    return {
      enabled: false,
      start: 0,
      end: totalRows,
      before: 0,
      after: 0,
    };
  }
  const viewportHeight = container.clientHeight || rowHeight * 20;
  const scrollTop = container.scrollTop || 0;
  const visibleRows = Math.ceil(viewportHeight / rowHeight);
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - VIRTUALIZATION_OVERSCAN);
  const end = Math.min(totalRows, start + visibleRows + VIRTUALIZATION_OVERSCAN * 2);
  return {
    enabled: true,
    start,
    end,
    before: start * rowHeight,
    after: Math.max(0, (totalRows - end) * rowHeight),
  };
}

function virtualSpacer(height) {
  const spacer = document.createElement("div");
  spacer.className = "virtual-spacer";
  spacer.style.height = `${Math.max(0, Math.round(height))}px`;
  spacer.setAttribute("aria-hidden", "true");
  return spacer;
}

function scheduleVirtualRender(state) {
  if (!state || state.frame) {
    return;
  }
  state.frame = window.requestAnimationFrame(() => {
    state.frame = null;
    state.render();
  });
}

function streamVirtualRowHeight() {
  return Math.max(STREAM_VIRTUAL_ROW_HEIGHT, (appearanceSettings?.monoFontSize ?? 12) + 6);
}

function streamVirtualRange(container, totalRows) {
  if (!container || totalRows <= 0) {
    return {
      start: 0,
      end: totalRows,
      before: 0,
      after: 0,
    };
  }

  const rowHeight = streamVirtualRowHeight();
  const viewportHeight = container.clientHeight || rowHeight * 20;
  const scrollTop = container.scrollTop || 0;
  const visibleRows = Math.ceil(viewportHeight / rowHeight);
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - STREAM_VIRTUAL_OVERSCAN);
  const end = Math.min(totalRows, start + visibleRows + STREAM_VIRTUAL_OVERSCAN * 2);
  return {
    start,
    end,
    before: start * rowHeight,
    after: Math.max(0, (totalRows - end) * rowHeight),
  };
}

function streamVirtualSpacer(height) {
  const spacer = document.createElement("div");
  spacer.className = "stream-virtual-spacer";
  spacer.style.height = `${Math.max(0, Math.round(height))}px`;
  spacer.setAttribute("aria-hidden", "true");
  return spacer;
}

class GuiTaskScheduler {
  constructor(kindLimits = {}) {
    this.kindLimits = kindLimits;
    this.queue = [];
    this.runningByKind = new Map();
    this.sequence = 0;
  }

  schedule(task) {
    const normalized = {
      priority: TASK_PRIORITY_USER,
      prefetch: false,
      coalesceKey: null,
      dropGroup: null,
      ...task,
      sequence: ++this.sequence,
    };
    if (normalized.dropGroup && !normalized.prefetch) {
      this.dropQueuedPrefetch(normalized.dropGroup);
    }
    if (normalized.coalesceKey) {
      const existing = this.queue.find((item) => item.coalesceKey === normalized.coalesceKey);
      if (existing) {
        return existing.promise;
      }
    }
    normalized.promise = new Promise((resolve, reject) => {
      normalized.resolve = resolve;
      normalized.reject = reject;
    });
    this.queue.push(normalized);
    this.sortQueue();
    this.pump();
    return normalized.promise;
  }

  sortQueue() {
    this.queue.sort((left, right) => right.priority - left.priority || left.sequence - right.sequence);
  }

  dropQueuedPrefetch(dropGroup) {
    const keep = [];
    for (const task of this.queue) {
      if (task.prefetch && task.dropGroup === dropGroup) {
        if (perfLogEnabled()) {
          console.debug(`[pdf-debugger perf] task_queue drop_prefetch kind=${task.kind} group=${dropGroup}`);
        }
        task.reject(new Error("Stale prefetch dropped."));
      } else {
        keep.push(task);
      }
    }
    this.queue = keep;
  }

  pump() {
    for (let index = 0; index < this.queue.length; index += 1) {
      const task = this.queue[index];
      if (task.prefetch && shouldPausePrefetchTasks()) {
        if (!task.pauseLogged && perfLogEnabled()) {
          console.debug(`[pdf-debugger perf] task_queue pause_prefetch kind=${task.kind}`);
          task.pauseLogged = true;
        }
        this.schedulePrefetchResume();
        continue;
      }
      const running = this.runningByKind.get(task.kind) ?? 0;
      const limit = this.kindLimits[task.kind] ?? 2;
      if (running >= limit) {
        continue;
      }
      this.queue.splice(index, 1);
      index -= 1;
      this.runningByKind.set(task.kind, running + 1);
      if (perfLogEnabled()) {
        console.debug(`[pdf-debugger perf] task_queue start kind=${task.kind} priority=${task.priority} queued=${this.queue.length}`);
      }
      Promise.resolve()
        .then(task.run)
        .then(task.resolve, task.reject)
        .finally(() => {
          this.runningByKind.set(task.kind, Math.max(0, (this.runningByKind.get(task.kind) ?? 1) - 1));
          if (perfLogEnabled()) {
            console.debug(`[pdf-debugger perf] task_queue finish kind=${task.kind} running=${this.runningByKind.get(task.kind) ?? 0}`);
          }
          this.pump();
        });
    }
  }

  schedulePrefetchResume() {
    if (this.prefetchResumeTimer != null) {
      return;
    }
    const elapsed = performance.now() - lastUserInteractionAt;
    const delay = Math.max(50, PREFETCH_INTERACTION_PAUSE_MS - elapsed);
    this.prefetchResumeTimer = window.setTimeout(() => {
      this.prefetchResumeTimer = null;
      this.pump();
    }, delay);
  }
}

const guiTaskScheduler = new GuiTaskScheduler(TASK_KIND_LIMITS);

function markUserInteraction() {
  lastUserInteractionAt = performance.now();
}

function shouldPausePrefetchTasks() {
  return performance.now() - lastUserInteractionAt < PREFETCH_INTERACTION_PAUSE_MS;
}

document.addEventListener("pointerdown", markUserInteraction, { capture: true, passive: true });
document.addEventListener("keydown", markUserInteraction, { capture: true });
document.addEventListener("wheel", markUserInteraction, { capture: true, passive: true });
document.addEventListener("contextmenu", (event) => {
  event.preventDefault();
});

function scheduleTauriTask(kind, command, args, options = {}) {
  const priority = options.prefetch ? TASK_PRIORITY_PREFETCH : TASK_PRIORITY_USER;
  return guiTaskScheduler.schedule({
    kind,
    priority: options.priority ?? priority,
    prefetch: Boolean(options.prefetch),
    coalesceKey: options.coalesceKey ?? `${kind}:${command}:${JSON.stringify(args)}`,
    dropGroup: options.dropGroup ?? kind,
    run: () => invoke(command, args),
  });
}

function initializeWorkspaceTabs() {
  workspaceTabs = [{ ...singletonTabDefinitions.overview }];
  activeWorkspaceTabId = "overview";
  renderWorkspaceTabs();
  applyActiveWorkspaceTab();
}

function setActiveWorkspaceTab(tabId) {
  const definition = singletonTabDefinitions[tabId];
  if (!definition) {
    return;
  }
  if (definition.panel !== "overview" && !isDocumentReady()) {
    showRecoverableNotice(
      isPdfLoading
        ? "Document is still loading. Workspaces become available after the PDF is fully parsed."
        : "Open a PDF before using this workspace.",
    );
    return false;
  }
  return openWorkspaceTab({ ...definition });
}

function openWorkspaceTab(tab) {
  if (tab?.panel !== "overview" && !isDocumentReady()) {
    showRecoverableNotice(
      isPdfLoading
        ? "Document is still loading. Workspaces become available after the PDF is fully parsed."
        : "Open a PDF before using this workspace.",
    );
    return false;
  }
  const existing = workspaceTabs.find((item) => item.id === tab.id);
  if (!existing) {
    workspaceTabs.push(tab);
  } else {
    Object.assign(existing, tab);
  }
  activeWorkspaceTabId = tab.id;
  renderWorkspaceTabs();
  applyActiveWorkspaceTab();
  return true;
}

function closeWorkspaceTab(tabId) {
  const index = workspaceTabs.findIndex((tab) => tab.id === tabId);
  if (index === -1 || workspaceTabs[index].closeable === false) {
    return;
  }

  workspaceTabs.splice(index, 1);
  if (activeWorkspaceTabId === tabId) {
    const nextTab = workspaceTabs[Math.max(0, index - 1)] ?? workspaceTabs[0] ?? singletonTabDefinitions.overview;
    if (!workspaceTabs.length) {
      workspaceTabs.push({ ...singletonTabDefinitions.overview });
    }
    activeWorkspaceTabId = nextTab.id;
  }
  renderWorkspaceTabs();
  applyActiveWorkspaceTab();
}

function closeActiveWorkspaceTab() {
  const tab = activeWorkspaceTab();
  if (!tab || tab.closeable === false) {
    return false;
  }
  closeWorkspaceTab(tab.id);
  return true;
}

function clearDocumentWorkspaceTabs() {
  workspaceTabs = [{ ...singletonTabDefinitions.overview }];
  activeWorkspaceTabId = "overview";
  renderWorkspaceTabs();
  applyActiveWorkspaceTab();
}

function renderWorkspaceTabs() {
  elements.workspaceTabs.replaceChildren();
  for (const tab of workspaceTabs) {
    const button = document.createElement("button");
    button.className = "workspace-tab";
    button.type = "button";
    button.role = "tab";
    button.dataset.tabId = tab.id;
    button.dataset.workspaceTab = tab.panel;
    button.setAttribute("aria-selected", String(tab.id === activeWorkspaceTabId));
    button.classList.toggle("is-active", tab.id === activeWorkspaceTabId);
    button.tabIndex = tab.id === activeWorkspaceTabId ? 0 : -1;
    button.title = tab.title;
    button.addEventListener("click", () => {
      activeWorkspaceTabId = tab.id;
      renderWorkspaceTabs();
      applyActiveWorkspaceTab();
    });

    const label = document.createElement("span");
    label.className = "workspace-tab-label";
    label.textContent = tab.title;
    button.appendChild(label);

    if (tab.closeable !== false) {
      const close = document.createElement("span");
      close.className = "workspace-tab-close";
      close.role = "button";
      close.tabIndex = 0;
      close.title = `Close ${tab.title}`;
      close.setAttribute("aria-label", `Close ${tab.title}`);
      close.textContent = "×";
      close.addEventListener("click", (event) => {
        event.stopPropagation();
        closeWorkspaceTab(tab.id);
      });
      close.addEventListener("keydown", (event) => {
        if (event.key !== "Enter" && event.key !== " ") {
          return;
        }
        event.preventDefault();
        event.stopPropagation();
        closeWorkspaceTab(tab.id);
      });
      button.appendChild(close);
    }

    elements.workspaceTabs.appendChild(button);
  }
  scheduleWorkspaceTabOverflowUpdate();
}

function scheduleWorkspaceTabOverflowUpdate() {
  requestAnimationFrame(updateWorkspaceTabOverflow);
}

function updateWorkspaceTabOverflow() {
  const hasOverflow = elements.workspaceTabs.scrollWidth > elements.workspaceTabs.clientWidth + 1;
  elements.workspaceTabOverflowButton.hidden = !hasOverflow;
  elements.workspaceTabOverflowButton.title = `More tabs (${workspaceTabs.length})`;
  elements.workspaceTabOverflowButton.setAttribute("aria-label", `More tabs (${workspaceTabs.length} open)`);

  if (!hasOverflow) {
    workspaceTabOverflowOpen = false;
    elements.workspaceTabOverflowButton.setAttribute("aria-expanded", "false");
    elements.workspaceTabOverflowMenu.hidden = true;
    elements.workspaceTabOverflowMenu.replaceChildren();
    return;
  }

  renderWorkspaceTabOverflowMenu();
  elements.workspaceTabOverflowButton.setAttribute("aria-expanded", String(workspaceTabOverflowOpen));
  elements.workspaceTabOverflowMenu.hidden = !workspaceTabOverflowOpen;
}

function setWorkspaceTabOverflowOpen(isOpen) {
  workspaceTabOverflowOpen = Boolean(isOpen) && !elements.workspaceTabOverflowButton.hidden;
  elements.workspaceTabOverflowButton.setAttribute("aria-expanded", String(workspaceTabOverflowOpen));
  elements.workspaceTabOverflowMenu.hidden = !workspaceTabOverflowOpen;

  if (!workspaceTabOverflowOpen) {
    return;
  }

  renderWorkspaceTabOverflowMenu();
  requestAnimationFrame(() => {
    const activeItem =
      elements.workspaceTabOverflowMenu.querySelector(".workspace-tab-overflow-item.is-active") ??
      elements.workspaceTabOverflowMenu.querySelector(".workspace-tab-overflow-item");
    activeItem?.focus();
  });
}

function renderWorkspaceTabOverflowMenu() {
  elements.workspaceTabOverflowMenu.replaceChildren();
  for (const tab of workspaceTabs) {
    const item = document.createElement("button");
    item.className = "workspace-tab-overflow-item";
    item.classList.toggle("is-active", tab.id === activeWorkspaceTabId);
    item.type = "button";
    item.role = "menuitemradio";
    item.title = tab.title;
    item.setAttribute("aria-checked", String(tab.id === activeWorkspaceTabId));
    item.addEventListener("click", () => {
      activeWorkspaceTabId = tab.id;
      workspaceTabOverflowOpen = false;
      renderWorkspaceTabs();
      applyActiveWorkspaceTab();
    });

    const title = document.createElement("span");
    title.className = "workspace-tab-overflow-title";
    title.textContent = tab.title;
    item.appendChild(title);

    if (tab.id === activeWorkspaceTabId) {
      const status = document.createElement("span");
      status.className = "workspace-tab-overflow-status";
      status.textContent = "Active";
      item.appendChild(status);
    }

    elements.workspaceTabOverflowMenu.appendChild(item);
  }
}

function activeWorkspaceTab() {
  return workspaceTabs.find((tab) => tab.id === activeWorkspaceTabId) ?? workspaceTabs[0];
}

function applyActiveWorkspaceTab() {
  const tab = activeWorkspaceTab();
  const panelId = tab?.panel ?? "overview";
  if (panelId !== "page") {
    setPageMetadataFloating(false);
  }

  for (const button of elements.navTabButtons) {
    button.classList.toggle("is-active", button.dataset.workspaceTab === panelId);
  }

  for (const panel of elements.workspaceTabPanels) {
    const isActive = panel.dataset.tabPanel === panelId;
    const wasHidden = panel.hidden;
    panel.hidden = !isActive;
    panel.classList.toggle("is-active", isActive);
    if (isActive && wasHidden) {
      restartPanelTransition(panel);
    }
  }

  if (tab?.kind === "object") {
    navigateToObject(tab.reference, { pushHistory: false, activateTab: false });
  } else if (tab?.kind === "page") {
    const pageNumber = tab.page?.page_number ?? tab.page?.pageNumber;
    scrollPageIntoVirtualView(Number(pageNumber));
    selectPage(tab.page, pageButtonForPage(tab.page), { activateTab: false });
  } else if (tab?.kind === "stream") {
    restoreStreamWorkspaceTab(tab.reference);
  } else if (tab?.kind === "singleton" && panelId === "acroform") {
    ensureAcroFormLoaded();
  } else if (tab?.kind === "singleton" && panelId === "annots") {
    renderAnnotations(activeAnnotations);
  }
}

function restartPanelTransition(panel) {
  restartElementTransition(panel);
  window.setTimeout(() => {
    panel.classList.remove("is-entering");
  }, 180);
}

function restartElementTransition(element) {
  element.classList.remove("is-entering");
  void element.offsetWidth;
  element.classList.add("is-entering");
}

function setPageMetadataFloating(isFloating) {
  if (!elements.pageMetadataPanel || !elements.pageMetadataPlaceholder) {
    return;
  }

  isPageMetadataFloating = Boolean(isFloating);
  elements.pageMetadataPanel.classList.toggle("is-floating", isPageMetadataFloating);
  elements.pageMetadataPlaceholder.hidden = !isPageMetadataFloating;
  if (!isPageMetadataFloating) {
    elements.pageMetadataPlaceholder.style.minHeight = "";
  }
  updatePageMetadataFloatButton();
  schedulePageMetadataFloatingLayoutSync();
}

function updatePageMetadataFloatButton() {
  if (!elements.pageMetadataFloat) {
    return;
  }

  const label = isPageMetadataFloating ? "Dock" : "Float";
  const title = isPageMetadataFloating ? "Dock page details" : "Float page details";
  elements.pageMetadataFloat.textContent = label;
  elements.pageMetadataFloat.title = title;
  elements.pageMetadataFloat.setAttribute("aria-label", `${title} panel`);
  elements.pageMetadataFloat.setAttribute("aria-pressed", String(isPageMetadataFloating));
}

function schedulePageMetadataFloatingLayoutSync() {
  if (pageMetadataFloatFrame != null) {
    return;
  }

  pageMetadataFloatFrame = window.requestAnimationFrame(() => {
    pageMetadataFloatFrame = null;
    syncPageMetadataFloatingLayout();
  });
}

function syncPageMetadataFloatingLayout() {
  if (!elements.pageMetadataPanel || !elements.pageMetadataPlaceholder) {
    return;
  }
  if (!isPageMetadataFloating) {
    elements.pageMetadataPlaceholder.style.minHeight = "";
    return;
  }

  const panelHeight = Math.ceil(elements.pageMetadataPanel.getBoundingClientRect().height);
  const boundedHeight = Math.max(1, Math.min(panelHeight, Math.max(1, window.innerHeight - 120)));
  elements.pageMetadataPlaceholder.style.minHeight = `${boundedHeight}px`;
}

function openObjectWorkspaceTab(reference, options = {}) {
  const normalized = normalizeReference(reference);
  const tabId = objectTabId(normalized);
  const opened = openWorkspaceTab({
    id: tabId,
    kind: "object",
    panel: "object",
    title: objectLabel(normalized),
    closeable: true,
    reference: normalized,
  });
  if (opened && options.pushHistory !== false) {
    pushNavigation(normalized);
  }
}

function openPageWorkspaceTab(page, button) {
  const pageNumber = page?.page_number ?? page?.pageNumber;
  if (!Number.isFinite(Number(pageNumber))) {
    setActiveWorkspaceTab("page");
    return;
  }

  openWorkspaceTab({
    id: `page:${Number(pageNumber)}`,
    kind: "page",
    panel: "page",
    title: `Page ${Number(pageNumber)}`,
    closeable: true,
    page,
    button,
  });
}

function openStreamWorkspaceTab(reference) {
  if (!reference) {
    return;
  }

  const normalized = normalizeReference(reference);
  inspectedStreamReference = normalized;
  openWorkspaceTab({
    id: "stream",
    kind: "stream",
    panel: "stream",
    title: `Stream ${objectLabel(normalized)}`,
    closeable: true,
    reference: normalized,
  });
}

function restoreStreamWorkspaceTab(reference) {
  if (!reference) {
    clearStreamViewer();
    return;
  }

  const normalized = normalizeReference(reference);
  inspectedStreamReference = normalized;
  if (!activeStreamView || !sameReference(activeStreamReference, normalized)) {
    loadStreamView(normalized);
    return;
  }

  elements.streamViewerSubtitle.textContent = objectLabel(normalized);
  renderStreamViewMetadata(activeStreamView);
  renderStreamViewerContent();
  loadActiveStreamPreview();
}

function objectTabId(reference) {
  return `object:${referenceKey(reference)}`;
}

function pageButtonForPage(page) {
  const pageNumber = page?.page_number ?? page?.pageNumber;
  if (!Number.isFinite(Number(pageNumber))) {
    return null;
  }
  return elements.pageList.querySelector(`[data-page-number="${Number(pageNumber)}"]`);
}

function scrollPageIntoVirtualView(pageNumber) {
  if (!Number.isFinite(Number(pageNumber)) || !activePages.length) {
    return;
  }
  const index = activePages.findIndex((page) => Number(page.page_number ?? page.pageNumber) === Number(pageNumber));
  if (index < 0) {
    return;
  }
  const top = index * PAGE_LIST_ROW_HEIGHT;
  const bottom = top + PAGE_LIST_ROW_HEIGHT;
  const viewportBottom = elements.pageList.scrollTop + elements.pageList.clientHeight;
  if (top < elements.pageList.scrollTop || bottom > viewportBottom) {
    elements.pageList.scrollTop = Math.max(0, top - PAGE_LIST_ROW_HEIGHT);
    renderVirtualPageList();
  }
}

function setLoading(isLoading) {
  isPdfLoading = Boolean(isLoading);
  elements.loadState.classList.toggle("is-loading", isLoading);
  if (isLoading) {
    elements.loadState.textContent = "Loading";
  } else if (elements.loadState.textContent === "Loading") {
    elements.loadState.textContent = "Ready";
  }
  elements.documentLoading.hidden = !isLoading;
  elements.documentLoading.setAttribute("aria-hidden", isLoading ? "false" : "true");
  for (const button of elements.openButtons) {
    button.disabled = isLoading;
  }
  updateDocumentInteractionState();
}

function setDocumentLoadingMessage(path) {
  const fileName = fileNameFromPath(path);
  elements.documentLoadingMessage.textContent = fileName
    ? `Loading and parsing ${fileName} before enabling the workspace...`
    : "Loading and parsing the complete PDF...";
  setDocumentOpenStatus("Loading PDF...");
  setPageListStatus("Waiting for full document load");
  setOverviewPreviewStatus("Not requested");
}

function showError(message) {
  elements.errorPanel.dataset.kind = "error";
  elements.errorPanelTitle.textContent = "Recoverable error";
  elements.errorMessage.textContent = message;
  elements.errorPanel.hidden = false;
  elements.loadState.textContent = "Error";
  setDocumentOpenStatus("Error");
}

function clearError() {
  delete elements.errorPanel.dataset.kind;
  elements.errorPanelTitle.textContent = "Recoverable error";
  elements.errorMessage.textContent = "";
  elements.errorPanel.hidden = true;
}

function showRecoverableNotice(message) {
  elements.errorPanel.dataset.kind = "error";
  elements.errorPanelTitle.textContent = "Recoverable issue";
  elements.errorMessage.textContent = message;
  elements.errorPanel.hidden = false;
  elements.loadState.textContent = "Ready";
}

function showInfoNotice(message) {
  elements.errorPanel.dataset.kind = "info";
  elements.errorPanelTitle.textContent = "Document ready";
  elements.errorMessage.textContent = message;
  elements.errorPanel.hidden = false;
  elements.loadState.textContent = "Ready";
}

function isDocumentReady() {
  return Boolean(currentPdfPath) && !isPdfLoading && currentOpenMode === "full";
}

function updateDocumentInteractionState() {
  const ready = isDocumentReady();
  for (const button of elements.navTabButtons) {
    const tab = button.dataset.workspaceTab;
    const requiresDocument = tab && tab !== "overview";
    const disabled = Boolean(requiresDocument && !ready);
    button.disabled = disabled;
    button.setAttribute("aria-disabled", String(disabled));
    button.classList.toggle("is-disabled", disabled);
    if (disabled) {
      button.title = isPdfLoading
        ? "Document operations are available after the PDF fully loads."
        : "Open a PDF before using this workspace.";
    } else {
      button.removeAttribute("title");
    }
  }
}

function setDocumentOpenStatus(message) {
  elements.documentOpenStatus.textContent = message;
}

function setPageListStatus(message) {
  elements.pageListStatus.textContent = message;
}

function setOverviewPreviewStatus(message) {
  elements.overviewPreviewStatus.textContent = message;
}

function setPagePreviewStatus(state, message, overviewMessage = null) {
  const labels = {
    idle: "Idle",
    queued: "Queued",
    rendering: "Rendering",
    loading: "Loading",
    complete: "Complete",
    "cache-hit": "Cache hit",
    stale: "Stale",
    failed: "Failed",
  };
  const normalized = labels[state] ? state : "idle";
  elements.pagePreviewState.textContent = labels[normalized];
  elements.pagePreviewState.dataset.state = normalized;
  elements.pagePreviewStatus.textContent = message;
  setOverviewPreviewStatus(overviewMessage ?? previewOverviewStatusForMessage(message));
}

function previewOverviewStatusForMessage(message) {
  const normalized = String(message ?? "").toLowerCase();
  if (!normalized || normalized.includes("select a page") || normalized.includes("open a pdf")) {
    return "Not requested";
  }
  if (normalized.includes("cache")) {
    return message;
  }
  if (normalized.includes("complete") || normalized.includes("rendered page")) {
    return message;
  }
  if (normalized.includes("queued")) {
    return message;
  }
  if (normalized.includes("stale") || normalized.includes("superseded")) {
    return message;
  }
  if (normalized.includes("unavailable") || normalized.includes("could not")) {
    return "Preview unavailable";
  }
  if (normalized.includes("rendering") || normalized.includes("loading")) {
    return message;
  }
  return "Not requested";
}

function resetSummary() {
  currentOpenMode = "none";
  updateWindowSubtitle(null);
  elements.fileName.textContent = "No PDF selected";
  elements.fileSize.textContent = "-";
  elements.pdfVersion.textContent = "-";
  elements.pageCount.textContent = "-";
  elements.objectCount.textContent = "-";
  elements.streamCount.textContent = "-";
  elements.xrefCount.textContent = "-";
  elements.encrypted.textContent = "-";
  elements.parseWarningCount.textContent = "-";
  setDocumentOpenStatus("No PDF open");
  setPageListStatus("Idle");
  setOverviewPreviewStatus("Not requested");
  activeTrailer = null;
  activeAcroForm = null;
  activeAnnotations = null;
  resetTrailerTreeState();
  resetObjectInspectionCache();
  renderTrailer(null);
  clearAcroForm();
  clearAnnotations();
}

function clearInspector() {
  selectedTreeButton?.classList.remove("is-selected");
  selectedTreeButton = null;
  elements.inspectorSubtitle.textContent = "Select an object from the tree";
  elements.inspectorState.textContent = "Idle";
  elements.inspectorError.hidden = true;
  elements.inspectorError.textContent = "";
  elements.inspectorReference.textContent = "-";
  elements.inspectorType.textContent = "-";
  elements.inspectorSummary.textContent = "-";
  elements.inspectorRange.textContent = "-";
  elements.inspectorRawLength.textContent = "-";
  elements.inspectorKeys.textContent = "-";
  elements.streamSection.hidden = true;
  elements.streamDeclaredLength.textContent = "-";
  elements.streamActualLength.textContent = "-";
  elements.streamFilters.textContent = "-";
  elements.streamDecodedLength.textContent = "-";
  elements.streamDecodeIssues.textContent = "-";
  elements.openStreamDetails.disabled = true;
  elements.openStreamDetails.title = "Select a stream object to inspect stream details.";
  inspectedStreamReference = null;
  activeObjectInspection = null;
  resetObjectDetailsTreeState();
  renderObjectDetailsTree(null);
  updateObjectEditToolbar();
  clearStreamViewer();
}

function clearPagePreview(message = "Open a PDF to populate pages.") {
  setPageMetadataFloating(false);
  activePages = [];
  selectedPageButton = null;
  virtualPageListState = null;
  elements.pageList.onscroll = null;
  clearSelectedPageObject();
  clearPageObjects("Select a page to populate page objects.");
  elements.pageListCount.textContent = "0";
  elements.pageList.classList.add("empty-list");
  elements.pageList.textContent = message;
  elements.pageMetadataSubtitle.textContent = "Select a page";
  elements.pageMetadataState.textContent = "Idle";
  renderPageMetadata(null);
  setPageListStatus(message);
  clearPagePreviewImage("Select a page to render a PDFium preview.");
}

function clearStreamViewer(message = "Select a stream object to inspect hex or decoded bytes.") {
  activeStreamView = null;
  activeStreamReference = null;
  closeStreamEditPanel();
  clearStreamImagePreview();
  clearContentAnalysis();
  streamViewRequestId += 1;
  elements.streamViewerSubtitle.textContent = "Select a stream object";
  elements.streamViewerState.textContent = "Idle";
  elements.streamViewerState.classList.remove("is-loading");
  elements.streamViewerError.hidden = true;
  elements.streamViewerError.textContent = "";
  clearStreamViewerStatus();
  elements.streamViewerReference.textContent = "-";
  elements.streamViewerRawLength.textContent = "-";
  elements.streamViewerByteRange.textContent = "-";
  elements.streamViewerDecodedLength.textContent = "-";
  elements.streamViewerFilters.textContent = "-";
  elements.streamViewerIssues.textContent = "-";
  setStreamViewerPlainText(message);
  setStreamMode("hex");
  updateStreamActionButtons();
  updateStreamEditControls();
  updateStreamImageControls();
}

function clearAcroForm(message = "Open a PDF to populate AcroForm fields.") {
  activeAcroForm = null;
  acroFormRequestId += 1;
  acroFormSearchQuery = "";
  if (elements.acroformSearch) {
    elements.acroformSearch.value = "";
  }
  elements.acroformSubtitle.textContent = "Open a PDF to inspect form fields";
  elements.acroformState.textContent = "Idle";
  elements.acroformState.classList.remove("is-loading");
  elements.acroformError.hidden = true;
  elements.acroformError.textContent = "";
  elements.acroformWarnings.hidden = true;
  elements.acroformWarnings.textContent = "";
  elements.acroformCount.textContent = "0";
  elements.acroformList.classList.add("empty-list");
  elements.acroformList.textContent = message;
}

function clearAnnotations(message = "Open a PDF to populate annotations.") {
  activeAnnotations = null;
  annotsSearchQuery = "";
  if (elements.annotsSearch) {
    elements.annotsSearch.value = "";
  }
  elements.annotsSubtitle.textContent = "Open a PDF to inspect annotations";
  elements.annotsState.textContent = "Idle";
  elements.annotsState.classList.remove("is-loading");
  elements.annotsError.hidden = true;
  elements.annotsError.textContent = "";
  elements.annotsWarnings.hidden = true;
  elements.annotsWarnings.textContent = "";
  elements.annotsCount.textContent = "0";
  elements.annotsList.classList.add("empty-list");
  elements.annotsList.textContent = message;
}

function clearContentAnalysis() {
  contentAnalysisRequestId += 1;
  activeContentTokens = [];
  activeContentOperators = [];
}

function clearObjectTreeSearch() {
  objectTreeSearchQuery = "";
  objectTreeSearchRows = [];
  if (elements.objectTreeSearch) {
    elements.objectTreeSearch.value = "";
  }
  updateObjectTreeSearchStatus();
}

function resetNavigationState() {
  navigationHistory = [];
  navigationIndex = -1;
  activeReference = null;
  navigationRequestId += 1;
  pageDetailsRequestId += 1;
  acroFormRequestId += 1;
}

function pushNavigation(reference) {
  const normalized = normalizeReference(reference);
  if (sameReference(navigationHistory[navigationIndex], normalized)) {
    return normalized;
  }

  navigationHistory = navigationHistory.slice(0, navigationIndex + 1);
  navigationHistory.push(normalized);
  navigationIndex = navigationHistory.length - 1;
  return normalized;
}

function setSelectedTreeReference(reference) {
  selectedTreeButton?.classList.remove("is-selected");
  const isSearching = objectTreeSearchQuery.trim().length > 0;
  if (isSearching) {
    refreshObjectTreeSearchRows();
    const targetIndex = objectTreeSearchRows.findIndex(
      (row) => row.node?.object && sameReference(row.node.object, reference),
    );
    if (targetIndex >= 0) {
      scrollObjectTreeIndexIntoView(targetIndex);
    } else {
      renderVirtualObjectTree();
    }
  } else {
    const targetPath = findObjectTreePathForReference(activeObjectTree, reference);
    if (targetPath) {
      expandObjectTreePath(targetPath);
      scrollObjectTreePathIntoView(targetPath);
    }
  }
  selectedTreeButton = treeButtonsByReference.get(referenceKey(reference)) ?? null;
  selectedTreeButton?.classList.add("is-selected");

  selectedTreeButton?.scrollIntoView({ block: "nearest" });
}

function renderSummary(summary) {
  const metadata = summary.metadata;
  currentOpenMode = summary.openMode ?? summary.open_mode ?? "full";
  if (currentOpenMode !== "full") {
    throw new Error("The GUI requires a fully loaded document before enabling inspection.");
  }
  isPdfLoading = false;

  elements.fileName.textContent = metadata.file_name ?? "Unknown PDF";
  elements.fileSize.textContent = formatBytes(metadata.file_size);
  elements.pdfVersion.textContent = metadata.pdf_version ?? "Unknown";
  elements.pageCount.textContent = metadata.page_count ?? "Unknown";
  elements.objectCount.textContent = metadata.object_count ?? 0;
  elements.streamCount.textContent = metadata.stream_count ?? 0;
  elements.xrefCount.textContent = metadata.xref_entry_count ?? 0;
  elements.encrypted.textContent = metadata.encrypted ? "Yes" : "No";
  elements.parseWarningCount.textContent = metadata.parse_warning_count ?? 0;
  currentPdfPath = summary.path;
  setDocumentOpenStatus("Open complete");
  setPageListStatus("Ready");
  setOverviewPreviewStatus("Not requested");
  resetNavigationState();
  resetTrailerTreeState();
  resetObjectInspectionCache();
  renderObjectTree(summary.objectTree);
  activeTrailer = summary.trailer ?? null;
  renderTrailer(activeTrailer, metadata.file_name);
  activeAcroForm = null;
  acroFormSearchQuery = "";
  if (elements.acroformSearch) {
    elements.acroformSearch.value = "";
  }
  activeAnnotations = summary.annotations ?? { annotations: [], warnings: [] };
  annotsSearchQuery = "";
  if (elements.annotsSearch) {
    elements.annotsSearch.value = "";
  }
  renderAcroForm(null, { fileName: metadata.file_name, idle: true });
  renderAnnotations(activeAnnotations, { fileName: metadata.file_name });
  summary.findings = [];
  summary.diagnostics = { errors: 0, warnings: 0, info: 0 };
  renderPageIndex(summary.pageIndex, { autoSelectFirst: false });
  clearInspector();
  if (activeWorkspaceTab()?.panel === "acroform") {
    ensureAcroFormLoaded();
  }
  if (activeWorkspaceTab()?.panel === "annots") {
    renderAnnotations(activeAnnotations, { fileName: metadata.file_name });
  }
  updateDocumentInteractionState();
}

function renderPageIndex(pageIndex, options = {}) {
  const autoSelectFirst = options.autoSelectFirst !== false;
  activePages = Array.isArray(pageIndex?.pages) ? pageIndex.pages : [];
  selectedPageButton = null;
  elements.pageList.replaceChildren();
  elements.pageList.onscroll = null;
  virtualPageListState = null;
  elements.pageListCount.textContent = activePages.length.toString();

  if (!activePages.length) {
    setPageListStatus("No pages discovered");
    clearPagePreview("No pages were discovered in the parsed page tree.");
    return;
  }

  elements.pageList.classList.remove("empty-list");
  setPageListStatus(`${activePages.length} page${activePages.length === 1 ? "" : "s"} ready`);
  renderVirtualPageList();
  elements.pageList.onscroll = () => scheduleVirtualRender(virtualPageListState);

  if (autoSelectFirst) {
    const firstButton = pageButtonForPage(activePages[0]);
    selectPage(activePages[0], firstButton, { activateTab: false });
  } else {
    activePage = null;
    renderPageMetadata(null);
    clearPageObjects("Select a page to populate page objects.");
    clearPagePreviewImage("Select a page to render a PDFium preview.");
    updatePagePreviewZoomControls();
  }
}

function selectPage(page, button, options = {}) {
  if (options.activateTab) {
    openPageWorkspaceTab(page, button);
    return;
  }

  const pageNumber = page?.page_number ?? page?.pageNumber;
  const activePageNumber = activePage?.page_number ?? activePage?.pageNumber;
  const previewMatchesPage = activePagePreview
    && Number(activePagePreview.pageNumber ?? activePagePreview.page_number) === Number(pageNumber)
    && samePreviewZoom(activePagePreview.zoom, pagePreviewZoom);
  const selectionWorkPending = Boolean(
    pageSelectionWorkTimer ||
    pageDetailsInFlight ||
    pageObjectsLoadTimer ||
    pagePreviewRenderTimer ||
    pendingPageDetailsLoad ||
    pageObjectsInFlight ||
    pagePreviewRenderInFlight ||
    pendingPageObjectsLoad ||
    pendingPagePreviewRender,
  );
  if (
    !options.force &&
    page &&
    Number(pageNumber) === Number(activePageNumber) &&
    selectedPageButton === (button ?? selectedPageButton) &&
    (selectionWorkPending || previewMatchesPage)
  ) {
    return;
  }

  cancelScheduledPageSelectionWork();
  cancelScheduledPageObjectLoad();
  cancelScheduledPagePreviewRender();
  activePage = page ?? null;
  if (page) {
    resetPagePreviewZoom({ rerender: false });
  }
  selectedPageButton?.classList.remove("is-selected");
  selectedPageButton = button ?? null;
  selectedPageButton?.classList.add("is-selected");
  clearSelectedPageObject();
  renderPageMetadata(page);
  scheduleSelectedPageDetails(page);
}

function scheduleSelectedPageDetails(page, delay = PAGE_SELECTION_SETTLE_MS) {
  if (pageSelectionWorkTimer != null) {
    window.clearTimeout(pageSelectionWorkTimer);
  }

  const pageNumber = page?.page_number ?? page?.pageNumber;
  const requestId = ++pageDetailsRequestId;
  elements.pageMetadataState.textContent = "Waiting";
  clearPageObjects("Waiting for page selection to settle...");
  clearPagePreviewImage("Waiting for page selection to settle...");
  if (!page || !Number.isFinite(Number(pageNumber))) {
    return;
  }

  pageSelectionWorkTimer = window.setTimeout(() => {
    pageSelectionWorkTimer = null;
    const selectedPageNumber = activePage?.page_number ?? activePage?.pageNumber;
    if (requestId !== pageDetailsRequestId || Number(selectedPageNumber) !== Number(pageNumber)) {
      return;
    }
    startSelectedPageDetailsLoad({ page, pageNumber: Number(pageNumber), requestId });
  }, delay);
}

function cancelScheduledPageSelectionWork() {
  if (pageSelectionWorkTimer == null) {
    return;
  }

  window.clearTimeout(pageSelectionWorkTimer);
  pageSelectionWorkTimer = null;
  pendingPageDetailsLoad = null;
}

function startSelectedPageDetailsLoad(task) {
  if (pageDetailsInFlight) {
    pendingPageDetailsLoad = task;
    elements.pageMetadataState.textContent = "Queued";
    return;
  }

  pageDetailsInFlight = true;
  loadSelectedPageDetails(task.page, task.requestId).finally(() => {
    pageDetailsInFlight = false;
    const nextTask = pendingPageDetailsLoad;
    pendingPageDetailsLoad = null;
    if (nextTask && nextTask.requestId === pageDetailsRequestId) {
      startSelectedPageDetailsLoad(nextTask);
    }
  });
}

async function loadSelectedPageDetails(page, requestId = ++pageDetailsRequestId) {
  const pageNumber = page?.page_number ?? page?.pageNumber;
  if (!currentPdfPath || !Number.isFinite(Number(pageNumber))) {
    schedulePageObjectsLoad(page, 0);
    schedulePagePreviewRender(0);
    return;
  }

  const selectedPageNumber = activePage?.page_number ?? activePage?.pageNumber;
  if (
    requestId !== pageDetailsRequestId ||
    Number(selectedPageNumber) !== Number(pageNumber)
  ) {
    return;
  }

  elements.pageMetadataState.textContent = "Ready";
  schedulePageObjectsLoad(page, 0);
  schedulePagePreviewRender(0);
}

function renderVirtualPageList() {
  const range = virtualRange(elements.pageList, activePages.length, PAGE_LIST_ROW_HEIGHT);
  const fragment = document.createDocumentFragment();
  if (range.enabled && range.before) {
    fragment.appendChild(virtualSpacer(range.before));
  }
  for (let index = range.start; index < range.end; index += 1) {
    fragment.appendChild(renderPageListItem(activePages[index]));
  }
  if (range.enabled && range.after) {
    fragment.appendChild(virtualSpacer(range.after));
  }
  elements.pageList.replaceChildren(fragment);
  selectedPageButton = activePage ? pageButtonForPage(activePage) : null;
  selectedPageButton?.classList.add("is-selected");
  virtualPageListState = {
    frame: null,
    render: renderVirtualPageList,
  };
}

function renderPageListItem(page) {
  const button = document.createElement("button");
  button.className = "page-list-item";
  button.type = "button";
  button.dataset.pageNumber = String(page.page_number ?? page.pageNumber ?? "");
  button.title = `Show metadata for page ${page.page_number ?? page.pageNumber ?? ""}`;
  button.addEventListener("click", () => openPageWorkspaceTab(page, button));

  const main = document.createElement("span");
  main.className = "page-list-main";
  main.textContent = `Page ${page.page_number ?? page.pageNumber ?? "-"}`;
  button.appendChild(main);

  const meta = document.createElement("span");
  meta.className = "page-list-meta";
  meta.textContent = `${objectLabel(page.reference)} · ${pageDimensions(page)}`;
  button.appendChild(meta);

  const resources = page.resources ?? {};
  const counts = document.createElement("span");
  counts.className = "page-list-counts";
  counts.textContent = `F ${resources.fonts ?? 0} · X ${resources.xobjects ?? resources.xObjects ?? 0} · I ${resources.images ?? 0}`;
  button.appendChild(counts);

  return button;
}

function prefetchAdjacentPageDetails(pageNumber) {
  return;
}

function renderPageMetadata(page) {
  const resources = page?.resources ?? {};
  elements.pageMetadataSubtitle.textContent = page
    ? `Page ${page.page_number ?? page.pageNumber ?? "-"}`
    : "Select a page";
  elements.pageMetadataState.textContent = page ? "Ready" : "Idle";
  elements.pageReference.textContent = page ? objectLabel(page.reference) : "-";
  elements.pageRotation.textContent = page?.rotation ?? "0";
  elements.pageMediaBox.textContent = formatPageBox(page?.media_box ?? page?.mediaBox);
  elements.pageCropBox.textContent = formatPageBox(page?.crop_box ?? page?.cropBox);
  elements.pageBleedBox.textContent = formatPageBox(page?.bleed_box ?? page?.bleedBox);
  elements.pageTrimBox.textContent = formatPageBox(page?.trim_box ?? page?.trimBox);
  elements.pageArtBox.textContent = formatPageBox(page?.art_box ?? page?.artBox);
  elements.pageResourceFonts.textContent = resources.fonts ?? 0;
  elements.pageResourceXobjects.textContent = resources.xobjects ?? resources.xObjects ?? 0;
  elements.pageResourceImages.textContent = resources.images ?? 0;
  elements.pageResourceContents.textContent = resources.contents ?? 0;
  elements.pageResourceAnnotations.textContent = resources.annotations ?? 0;
  renderPageObjectLinks(page?.links);
  schedulePageMetadataFloatingLayoutSync();
}

function renderPageObjectLinks(links) {
  activePageLinks = Array.isArray(links) ? links : [];
  pageReferenceSearchQuery = "";
  if (elements.pageReferenceSearch) {
    elements.pageReferenceSearch.value = "";
  }
  renderFilteredPageObjectLinks();
}

function renderFilteredPageObjectLinks() {
  elements.pageObjectLinks.replaceChildren();
  const links = activePageLinks.filter((link) => link?.reference);
  elements.pageLinksSection.hidden = links.length === 0;
  if (elements.pageReferenceCount) {
    elements.pageReferenceCount.textContent = String(links.length);
  }
  if (!links.length) {
    elements.pageObjectLinks.classList.remove("empty-list");
    return;
  }

  const query = pageReferenceSearchQuery.trim().toLowerCase();
  const filteredLinks = query
    ? links.filter((link) => pageReferenceSearchText(link).includes(query))
    : links;

  if (elements.pageReferenceCount) {
    elements.pageReferenceCount.textContent = query
      ? `${filteredLinks.length}/${links.length}`
      : String(links.length);
  }

  if (!filteredLinks.length) {
    elements.pageObjectLinks.classList.add("empty-list");
    elements.pageObjectLinks.textContent = "No page references match the search.";
    schedulePageMetadataFloatingLayoutSync();
    return;
  }

  elements.pageObjectLinks.classList.remove("empty-list");
  for (const link of filteredLinks) {
    const reference = link.reference;
    const referenceText = objectLabel(reference);
    const labelText = link.label ?? "Object";

    const button = document.createElement("button");
    button.className = "reference-list-row";
    button.type = "button";
    button.title = `Open ${referenceText}`;
    button.addEventListener("click", () => openObjectWorkspaceTab(reference));

    const label = document.createElement("span");
    label.className = "reference-list-label";
    label.textContent = labelText;
    button.appendChild(label);

    const value = document.createElement("span");
    value.className = "reference-list-value";
    value.textContent = referenceText;
    button.appendChild(value);

    elements.pageObjectLinks.appendChild(button);
  }
  schedulePageMetadataFloatingLayoutSync();
}

function pageReferenceSearchText(link) {
  const referenceText = link?.reference ? objectLabel(link.reference) : "";
  return `${link?.label ?? "Object"} ${referenceText}`.toLowerCase();
}

function schedulePageObjectsLoad(page, delay = PAGE_OBJECTS_LOAD_DEBOUNCE_MS) {
  cancelScheduledPageObjectLoad();
  const pageNumber = page?.page_number ?? page?.pageNumber;
  const requestId = ++pageObjectsRequestId;
  if (!currentPdfPath || !Number.isFinite(Number(pageNumber))) {
    clearPageObjects("Select a page to populate page objects.");
    return;
  }
  setPageObjectsPending("Waiting to inspect page objects...");
  pageObjectsLoadTimer = window.setTimeout(() => {
    pageObjectsLoadTimer = null;
    const selectedPageNumber = activePage?.page_number ?? activePage?.pageNumber;
    if (requestId !== pageObjectsRequestId || Number(selectedPageNumber) !== Number(pageNumber)) {
      return;
    }

    startPageObjectsLoad({ page, pageNumber: Number(pageNumber), requestId });
  }, delay);
}

function cancelScheduledPageObjectLoad() {
  if (pageObjectsLoadTimer != null) {
    window.clearTimeout(pageObjectsLoadTimer);
    pageObjectsLoadTimer = null;
  }
  pendingPageObjectsLoad = null;
}

function startPageObjectsLoad(task) {
  if (pageObjectsInFlight) {
    pendingPageObjectsLoad = task;
    elements.pageObjectsStatus.textContent = `Queued page ${task.pageNumber} object inspection...`;
    return;
  }

  pageObjectsInFlight = true;
  loadPageObjects(task.page, task.requestId).finally(() => {
    pageObjectsInFlight = false;
    const nextTask = pendingPageObjectsLoad;
    pendingPageObjectsLoad = null;
    if (nextTask && nextTask.requestId === pageObjectsRequestId) {
      startPageObjectsLoad(nextTask);
    }
  });
}

async function loadPageObjects(page, requestId = ++pageObjectsRequestId) {
  const pageNumber = page?.page_number ?? page?.pageNumber;
  setPageObjectsPending("Loading page objects...");
  if (!currentPdfPath || !Number.isFinite(Number(pageNumber))) {
    clearPageObjects("Select a page to populate page objects.");
    return;
  }
  try {
    const inspection = await scheduleTauriTask(
      "pageObjects",
      "inspect_page_objects",
      {
        path: currentPdfPath,
        pageNumber: Number(pageNumber),
      },
      {
        dropGroup: "pageObjects",
        coalesceKey: `pageObjects:${currentPdfPath}:${Number(pageNumber)}`,
      },
    );
    const selectedPageNumber = activePage?.page_number ?? activePage?.pageNumber;
    if (requestId !== pageObjectsRequestId || Number(selectedPageNumber) !== Number(pageNumber)) {
      return;
    }
    renderPageObjects(inspection);
  } catch (error) {
    if (requestId !== pageObjectsRequestId) {
      return;
    }
    clearPageObjects(`Page objects are unavailable: ${String(error)}`);
  }
}

function setPageObjectsPending(message) {
  activePageObjectInspection = null;
  activePageObjects = [];
  elements.pageObjectsStatus.textContent = message;
  elements.pageObjectsList.classList.add("empty-list");
  elements.pageObjectsList.textContent = message;
  elements.pageObjectWarningsSection.hidden = true;
  elements.pageObjectWarnings.replaceChildren();
  clearSelectedPageObject();
  schedulePageMetadataFloatingLayoutSync();
}

function clearPageObjects(message = "Select a page to populate page objects.") {
  pageObjectsRequestId += 1;
  setPageObjectsPending(message);
}

function renderPageObjects(inspection) {
  activePageObjectInspection = inspection;
  activePageObjects = Array.isArray(inspection?.objects) ? inspection.objects : [];
  elements.pageObjectsStatus.textContent = `${activePageObjects.length} page object(s) extracted.`;
  elements.pageObjectsList.replaceChildren();
  elements.pageObjectsList.classList.toggle("empty-list", activePageObjects.length === 0);

  if (!activePageObjects.length) {
    elements.pageObjectsList.textContent = "No supported page objects were extracted from this page.";
  }

  for (const object of activePageObjects) {
    const button = document.createElement("button");
    button.className = "page-object-row";
    button.type = "button";
    button.dataset.pageObjectId = object.id;
    button.title = object.summary ?? object.label ?? object.id;
    button.addEventListener("click", () => selectPageObject(object, button));

    const icon = document.createElement("span");
    icon.className = `page-object-kind-icon kind-${object.kind ?? "unknown"}`;
    icon.textContent = pageObjectKindIcon(object.kind);
    button.appendChild(icon);

    const copy = document.createElement("span");
    copy.className = "page-object-copy";
    const label = document.createElement("span");
    label.className = "page-object-label";
    label.textContent = object.label ?? object.id;
    copy.appendChild(label);
    const meta = document.createElement("span");
    meta.className = "page-object-meta";
    meta.textContent = pageObjectMeta(object);
    copy.appendChild(meta);
    button.appendChild(copy);

    elements.pageObjectsList.appendChild(button);
  }

  renderPageObjectWarnings(inspection?.warnings ?? []);
  clearSelectedPageObject();
  schedulePageMetadataFloatingLayoutSync();
}

function renderPageObjectWarnings(warnings) {
  elements.pageObjectWarnings.replaceChildren();
  elements.pageObjectWarningsSection.hidden = !warnings.length;
  for (const warning of warnings) {
    const item = document.createElement("div");
    item.className = "content-warning";
    const title = document.createElement("div");
    title.className = "content-warning-title";
    title.textContent = warning.ruleId ?? "page_object.warning";
    item.appendChild(title);
    const message = document.createElement("div");
    message.className = "content-warning-message";
    message.textContent = warning.message ?? "";
    item.appendChild(message);
    elements.pageObjectWarnings.appendChild(item);
  }
}

function selectPageObject(object, row) {
  selectedPageObjectRow?.classList.remove("is-selected");
  selectedPageObjectRow = row ?? elements.pageObjectsList.querySelector(`[data-page-object-id="${object.id}"]`);
  selectedPageObjectRow?.classList.add("is-selected");
  selectedPageObject = object ?? null;
  elements.pageSelectedObjectEmpty.textContent = "Select a page object to inspect properties.";
  renderSelectedPageObject(object);
  showPageObjectOverlay(object);
}

function renderSelectedPageObject(object) {
  if (!object) {
    clearSelectedPageObject();
    return;
  }

  elements.pageSelectedObjectEmpty.hidden = true;
  elements.pageSelectedObjectTable.hidden = false;
  elements.pageSelectedObjectProperties.replaceChildren();

  const rows = [
    ["Kind", pageObjectKindLabel(object.kind)],
    ["Label", object.label ?? "-"],
    ["Summary", object.summary ?? "-"],
    ["BBox", formatPageObjectBounds(object.bbox)],
    ["Coordinate space", object.bbox?.coordinateSpace ?? "-"],
    ["Reference", object.reference ? objectLabel(object.reference) : "-"],
    ["Content stream", object.contentStream ? objectLabel(object.contentStream) : "-"],
    ["Byte range", object.byteRange ? `${object.byteRange.start}..${object.byteRange.end}` : "-"],
  ];

  for (const property of object.properties ?? []) {
    rows.push([property.name, property.value]);
  }
  for (const warning of object.warnings ?? []) {
    rows.push(["Warning", warning]);
  }

  for (const [name, value] of rows) {
    const tr = document.createElement("tr");
    const th = document.createElement("th");
    th.textContent = name;
    tr.appendChild(th);
    const td = document.createElement("td");
    td.textContent = value || "-";
    tr.appendChild(td);
    elements.pageSelectedObjectProperties.appendChild(tr);
  }

  elements.pageSelectedObjectNote.textContent = object.bbox
    ? "Preview overlay maps PDF-space bounds to the rendered image as a best-effort display coordinate."
    : "This object does not have a usable bbox in the current first-step extractor.";
  elements.pageSelectedObjectOpen.disabled = !object.reference;
  schedulePageMetadataFloatingLayoutSync();
}

function clearSelectedPageObject() {
  selectedPageObjectRow?.classList.remove("is-selected");
  selectedPageObject = null;
  selectedPageObjectRow = null;
  elements.pageSelectedObjectEmpty.textContent = "Select a page object to inspect properties.";
  elements.pageSelectedObjectEmpty.hidden = false;
  elements.pageSelectedObjectTable.hidden = true;
  elements.pageSelectedObjectProperties.replaceChildren();
  elements.pageSelectedObjectNote.textContent = "";
  elements.pageSelectedObjectOpen.disabled = true;
  clearPageObjectOverlay();
  schedulePageMetadataFloatingLayoutSync();
}

function clearSelectedPageObjectFromPreview(message) {
  clearSelectedPageObject();
  elements.pageSelectedObjectEmpty.textContent = message;
  elements.pageSelectedObjectNote.textContent = message;
}

function showPageObjectOverlay(object) {
  if (!object?.bbox || elements.pagePreviewFigure.hidden) {
    clearPageObjectOverlay();
    return;
  }

  const layout = previewOverlayLayout(object.bbox);
  if (!layout) {
    clearPageObjectOverlay();
    return;
  }

  elements.pageObjectOverlay.style.left = `${layout.left}%`;
  elements.pageObjectOverlay.style.top = `${layout.top}%`;
  elements.pageObjectOverlay.style.width = `${layout.width}%`;
  elements.pageObjectOverlay.style.height = `${layout.height}%`;
  elements.pageObjectOverlay.dataset.pageObjectId = object.id;
  elements.pageObjectOverlayLabel.textContent = object.label ?? object.id;
  elements.pageObjectOverlay.title = `Best-effort bbox for ${object.label ?? object.id}`;
  elements.pageObjectOverlay.hidden = false;
}

function clearPageObjectOverlay() {
  elements.pageObjectOverlay.hidden = true;
  elements.pageObjectOverlay.removeAttribute("style");
  delete elements.pageObjectOverlay.dataset.pageObjectId;
  elements.pageObjectOverlayLabel.textContent = "Selected object";
}

function setPagePreviewZoom(zoom, options = {}) {
  const nextZoom = clampNumber(zoom, PAGE_PREVIEW_MIN_ZOOM, PAGE_PREVIEW_MAX_ZOOM);
  if (nextZoom === pagePreviewZoom) {
    updatePagePreviewZoomControls();
    return;
  }

  const anchor = options.anchor === undefined ? capturePagePreviewViewportCenterAnchor() : options.anchor;
  pagePreviewZoom = nextZoom;
  updatePagePreviewZoomControls();
  scheduleApplyPagePreviewZoom(anchor);
  if (options.rerender === false) {
    cancelScheduledPagePreviewRender();
    return;
  }

  if (activePage && currentPdfPath) {
    schedulePagePreviewRender(PAGE_PREVIEW_RENDER_DEBOUNCE_MS, anchor);
  }
}

function adjustPagePreviewZoom(delta, options = {}) {
  setPagePreviewZoom(pagePreviewZoom + delta, options);
}

function resetPagePreviewZoom(options = {}) {
  setPagePreviewZoom(1, options);
}

function updatePagePreviewZoomControls() {
  const hasPreview = Boolean(activePage || activePagePreview);
  elements.pageZoomLabel.textContent = `${Math.round(pagePreviewZoom * 100)}%`;
  elements.pageZoomOut.disabled = !hasPreview || pagePreviewZoom <= PAGE_PREVIEW_MIN_ZOOM;
  elements.pageZoomIn.disabled = !hasPreview || pagePreviewZoom >= PAGE_PREVIEW_MAX_ZOOM;
  elements.pageZoomReset.disabled = !hasPreview || pagePreviewZoom === 1;
}

function scheduleApplyPagePreviewZoom(anchor = null) {
  if (anchor) {
    pendingPagePreviewZoomAnchor = anchor;
    lastPagePreviewZoomAnchor = anchor;
  }
  if (pagePreviewZoomFrame != null) {
    return;
  }

  pagePreviewZoomFrame = window.requestAnimationFrame(() => {
    const frameAnchor = pendingPagePreviewZoomAnchor;
    pendingPagePreviewZoomAnchor = null;
    pagePreviewZoomFrame = null;
    applyPagePreviewZoom(frameAnchor);
  });
}

function cancelScheduledPagePreviewZoomFrame() {
  if (pagePreviewZoomFrame == null) {
    return;
  }

  window.cancelAnimationFrame(pagePreviewZoomFrame);
  pagePreviewZoomFrame = null;
}

function cancelPagePreviewSwapFrame() {
  if (pagePreviewSwapFrame == null) {
    return;
  }

  window.cancelAnimationFrame(pagePreviewSwapFrame);
  pagePreviewSwapFrame = null;
}

function withInstantPagePreviewSwap(callback) {
  cancelScheduledPagePreviewZoomFrame();
  cancelPagePreviewSwapFrame();
  elements.pagePreviewStage.classList.add("is-preview-swap-instant");
  void elements.pagePreviewStage.offsetWidth;
  try {
    callback();
    void elements.pagePreviewStage.offsetWidth;
    pagePreviewSwapFrame = window.requestAnimationFrame(() => {
      pagePreviewSwapFrame = null;
      elements.pagePreviewStage.classList.remove("is-preview-swap-instant");
    });
  } catch (error) {
    elements.pagePreviewStage.classList.remove("is-preview-swap-instant");
    throw error;
  }
}

function clearPagePreviewStageLayout() {
  cancelScheduledPagePreviewZoomFrame();
  cancelPagePreviewSwapFrame();
  pendingPagePreviewZoomAnchor = null;
  pagePreviewRenderAnchor = null;
  lastPagePreviewZoomAnchor = null;
  elements.pagePreviewStage.classList.remove("is-preview-swap-instant");
  elements.pagePreviewStage.removeAttribute("style");
  clearPagePreviewCanvasSize();
}

function applyPagePreviewZoom(anchor = null) {
  if (!activePagePreview || elements.pagePreviewFigure.hidden) {
    clearPagePreviewStageLayout();
    return;
  }

  const width = Number(activePagePreview.pixelWidth);
  const height = Number(activePagePreview.pixelHeight);
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    return;
  }

  elements.pagePreviewStage.style.width = `${Math.max(1, Math.round(width))}px`;
  elements.pagePreviewStage.style.height = `${Math.max(1, Math.round(height))}px`;
  const renderedZoom = normalizedPreviewZoom(activePagePreview.zoom);
  const visualScale = pagePreviewZoom / renderedZoom;
  elements.pagePreviewStage.style.transformOrigin = anchor
    ? `${clampNumber(anchor.relativeX, 0, 1) * 100}% ${clampNumber(anchor.relativeY, 0, 1) * 100}%`
    : "center center";
  elements.pagePreviewStage.style.transform = `scale3d(${visualScale}, ${visualScale}, 1)`;
  applyPagePreviewCanvasSize(width * visualScale, height * visualScale, {
    anchor,
    layoutHeight: height,
    layoutWidth: width,
  });
  applyPagePreviewZoomAnchor(anchor);
  if (selectedPageObject) {
    showPageObjectOverlay(selectedPageObject);
  }
}

function schedulePagePreviewRender(delay = PAGE_PREVIEW_RENDER_DEBOUNCE_MS, anchor = null) {
  cancelScheduledPagePreviewRender();
  if (!activePage || !currentPdfPath) {
    return;
  }
  const pageNumber = activePage?.page_number ?? activePage?.pageNumber;
  if (!Number.isFinite(Number(pageNumber))) {
    return;
  }
  if (
    activePagePreview &&
    Number(activePagePreview.pageNumber ?? activePagePreview.page_number) === Number(pageNumber) &&
    samePreviewZoom(activePagePreview.zoom, pagePreviewZoom) &&
    pagePreviewRenderZoom == null
  ) {
    return;
  }

  if (anchor) {
    pagePreviewRenderAnchor = anchor;
  }
  pagePreviewRenderTimer = window.setTimeout(() => {
    pagePreviewRenderTimer = null;
    startPagePreviewRender({
      page: activePage,
      anchor: pagePreviewRenderAnchor,
      zoom: pagePreviewZoom,
      requestId: ++pagePreviewRequestId,
    });
  }, delay);
}

function cancelScheduledPagePreviewRender() {
  if (pagePreviewRenderTimer == null) {
    pendingPagePreviewRender = null;
    return;
  }

  window.clearTimeout(pagePreviewRenderTimer);
  pagePreviewRenderTimer = null;
  pagePreviewRenderAnchor = null;
  pendingPagePreviewRender = null;
}

function startPagePreviewRender(task) {
  if (pagePreviewRenderInFlight) {
    pendingPagePreviewRender = task;
    const pageNumber = task.page?.page_number ?? task.page?.pageNumber;
    setPagePreviewStatus(
      "queued",
      `Queued page ${pageNumber} render at ${Math.round(normalizedPreviewZoom(task.zoom) * 100)}%.`,
      `Queued page ${pageNumber}`,
    );
    return;
  }

  pagePreviewRenderInFlight = true;
  loadPagePreview(task.page, {
    anchor: task.anchor,
    preserveCurrent: true,
    requestId: task.requestId,
    zoom: task.zoom,
  }).finally(() => {
    pagePreviewRenderInFlight = false;
    const nextTask = pendingPagePreviewRender;
    pendingPagePreviewRender = null;
    if (nextTask && nextTask.requestId === pagePreviewRequestId) {
      startPagePreviewRender(nextTask);
    }
  });
}

function normalizedPreviewZoom(zoom) {
  return clampNumber(Number(zoom), PAGE_PREVIEW_MIN_ZOOM, PAGE_PREVIEW_MAX_ZOOM);
}

function samePreviewZoom(left, right) {
  return Math.abs(normalizedPreviewZoom(left) - normalizedPreviewZoom(right)) < 0.001;
}

function applyPagePreviewCanvasSize(width, height, options = {}) {
  const viewport = elements.pagePreviewViewport;
  const canvas = elements.pagePreviewCanvas;
  if (!viewport || !canvas) {
    return;
  }

  const anchor = options.anchor;
  const layoutWidth = Number(options.layoutWidth);
  const layoutHeight = Number(options.layoutHeight);
  const visualWidth = Math.max(1, Number(width));
  const visualHeight = Math.max(1, Number(height));
  const baseWidth = Math.max(visualWidth, Number.isFinite(layoutWidth) ? layoutWidth : 0);
  const baseHeight = Math.max(visualHeight, Number.isFinite(layoutHeight) ? layoutHeight : 0);
  const anchoredWidth = anchor ? baseWidth + viewport.clientWidth * 2 : baseWidth;
  const anchoredHeight = anchor ? baseHeight + viewport.clientHeight * 2 : baseHeight;
  const canvasWidth = Math.max(1, Math.round(anchoredWidth), viewport.clientWidth);
  const canvasHeight = Math.max(1, Math.round(anchoredHeight), viewport.clientHeight);
  canvas.style.width = `${canvasWidth}px`;
  canvas.style.height = `${canvasHeight}px`;
}

function clearPagePreviewCanvasSize() {
  elements.pagePreviewCanvas?.removeAttribute("style");
}

function capturePagePreviewPointerAnchor(clientX, clientY) {
  return createPagePreviewZoomAnchor(clientX, clientY)
    ?? capturePagePreviewViewportCenterAnchor();
}

function capturePagePreviewViewportCenterAnchor() {
  const viewport = elements.pagePreviewViewport;
  if (!viewport) {
    return null;
  }

  const viewportRect = viewport.getBoundingClientRect();
  if (!viewportRect.width || !viewportRect.height) {
    return null;
  }

  return createPagePreviewZoomAnchor(
    viewportRect.left + viewportRect.width / 2,
    viewportRect.top + viewportRect.height / 2,
    { clampToStage: true },
  );
}

function createPagePreviewZoomAnchor(clientX, clientY, options = {}) {
  const viewport = elements.pagePreviewViewport;
  if (!viewport || !activePagePreview || elements.pagePreviewFigure.hidden) {
    return null;
  }

  const viewportRect = viewport.getBoundingClientRect();
  const stageRect = elements.pagePreviewStage.getBoundingClientRect();
  if (!viewportRect.width || !viewportRect.height || !stageRect.width || !stageRect.height) {
    return null;
  }

  const viewportX = clientX - viewportRect.left;
  const viewportY = clientY - viewportRect.top;
  const pointerInViewport = viewportX >= 0
    && viewportX <= viewportRect.width
    && viewportY >= 0
    && viewportY <= viewportRect.height;
  if (!pointerInViewport && !options.clampToStage) {
    return null;
  }

  const rawRelativeX = (clientX - stageRect.left) / stageRect.width;
  const rawRelativeY = (clientY - stageRect.top) / stageRect.height;
  const pointerInStage = rawRelativeX >= 0
    && rawRelativeX <= 1
    && rawRelativeY >= 0
    && rawRelativeY <= 1;
  if (!pointerInStage && !options.clampToStage) {
    return null;
  }

  return {
    relativeX: clampNumber(rawRelativeX, 0, 1),
    relativeY: clampNumber(rawRelativeY, 0, 1),
    viewportX: clampNumber(viewportX, 0, viewportRect.width),
    viewportY: clampNumber(viewportY, 0, viewportRect.height),
  };
}

function applyPagePreviewZoomAnchor(anchor) {
  const viewport = elements.pagePreviewViewport;
  if (!anchor || !viewport || elements.pagePreviewFigure.hidden) {
    return;
  }

  const viewportRect = viewport.getBoundingClientRect();
  const stageRect = elements.pagePreviewStage.getBoundingClientRect();
  if (!viewportRect.width || !viewportRect.height || !stageRect.width || !stageRect.height) {
    return;
  }

  const targetX = viewportRect.left + clampNumber(anchor.viewportX, 0, viewportRect.width);
  const targetY = viewportRect.top + clampNumber(anchor.viewportY, 0, viewportRect.height);
  const currentX = stageRect.left + clampNumber(anchor.relativeX, 0, 1) * stageRect.width;
  const currentY = stageRect.top + clampNumber(anchor.relativeY, 0, 1) * stageRect.height;
  const nextScrollLeft = viewport.scrollLeft + currentX - targetX;
  const nextScrollTop = viewport.scrollTop + currentY - targetY;
  viewport.scrollLeft = clampNumber(nextScrollLeft, 0, Math.max(0, viewport.scrollWidth - viewport.clientWidth));
  viewport.scrollTop = clampNumber(nextScrollTop, 0, Math.max(0, viewport.scrollHeight - viewport.clientHeight));
}

function handlePagePreviewWheel(event) {
  if (!event.ctrlKey) {
    return;
  }

  event.preventDefault();
  const direction = event.deltaY < 0 ? 1 : -1;
  const anchor = capturePagePreviewPointerAnchor(event.clientX, event.clientY);
  adjustPagePreviewZoom(direction * PAGE_PREVIEW_WHEEL_ZOOM_STEP, { anchor });
}

function handlePagePreviewClick(event) {
  if (event.target === elements.pageObjectOverlay || elements.pageObjectOverlay.contains(event.target)) {
    return;
  }
  if (!activePageObjects.some((object) => object?.bbox)) {
    clearSelectedPageObjectFromPreview("No clickable page object bbox is available for this page.");
    return;
  }
  const point = pageCoordinateFromPreviewEvent(event);
  if (!point) {
    clearSelectedPageObjectFromPreview("Preview coordinate mapping is unavailable for this click.");
    return;
  }

  const hit = hitTestPageObjects(point);
  if (!hit) {
    clearSelectedPageObjectFromPreview("No object at this position.");
    return;
  }

  const row = elements.pageObjectsList.querySelector(`[data-page-object-id="${hit.id}"]`);
  selectPageObject(hit, row);
  row?.scrollIntoView({ block: "nearest" });
}

function pageCoordinateFromPreviewEvent(event) {
  const pageBox = activePageObjectInspection?.pageBox ?? activePage?.media_box ?? activePage?.mediaBox;
  if (!pageBox || elements.pagePreviewFigure.hidden) {
    return null;
  }

  const rect = elements.pagePreviewStage.getBoundingClientRect();
  if (!rect.width || !rect.height) {
    return null;
  }

  const relativeX = (event.clientX - rect.left) / rect.width;
  const relativeY = (event.clientY - rect.top) / rect.height;
  if (relativeX < 0 || relativeX > 1 || relativeY < 0 || relativeY > 1) {
    return null;
  }

  const pageLeft = Number(pageBox.lower_left_x ?? pageBox.lowerLeftX ?? 0);
  const pageBottom = Number(pageBox.lower_left_y ?? pageBox.lowerLeftY ?? 0);
  const pageWidth = Number(pageBox.width);
  const pageHeight = Number(pageBox.height);
  if (!Number.isFinite(pageWidth) || !Number.isFinite(pageHeight) || pageWidth <= 0 || pageHeight <= 0) {
    return null;
  }

  return {
    x: pageLeft + relativeX * pageWidth,
    y: pageBottom + (1 - relativeY) * pageHeight,
  };
}

function hitTestPageObjects(point) {
  const hits = activePageObjects
    .filter((object) => boundsContainPoint(object.bbox, point))
    .sort((left, right) => boundsArea(left.bbox) - boundsArea(right.bbox));
  return hits[0] ?? null;
}

function boundsContainPoint(bounds, point) {
  if (!bounds || !point) {
    return false;
  }
  const left = Number(bounds.lowerLeftX ?? bounds.lower_left_x);
  const right = Number(bounds.upperRightX ?? bounds.upper_right_x);
  const bottom = Number(bounds.lowerLeftY ?? bounds.lower_left_y);
  const top = Number(bounds.upperRightY ?? bounds.upper_right_y);
  if (![left, right, bottom, top].every(Number.isFinite)) {
    return false;
  }

  return point.x >= Math.min(left, right)
    && point.x <= Math.max(left, right)
    && point.y >= Math.min(bottom, top)
    && point.y <= Math.max(bottom, top);
}

function boundsArea(bounds) {
  const width = Number(bounds?.width);
  const height = Number(bounds?.height);
  if (Number.isFinite(width) && Number.isFinite(height) && width >= 0 && height >= 0) {
    return width * height;
  }
  const left = Number(bounds?.lowerLeftX ?? bounds?.lower_left_x);
  const right = Number(bounds?.upperRightX ?? bounds?.upper_right_x);
  const bottom = Number(bounds?.lowerLeftY ?? bounds?.lower_left_y);
  const top = Number(bounds?.upperRightY ?? bounds?.upper_right_y);
  if (![left, right, bottom, top].every(Number.isFinite)) {
    return Number.POSITIVE_INFINITY;
  }
  return Math.abs(right - left) * Math.abs(top - bottom);
}

function previewOverlayLayout(bounds) {
  const pageBox = activePageObjectInspection?.pageBox ?? activePage?.media_box ?? activePage?.mediaBox;
  if (!pageBox || !bounds) {
    return null;
  }
  const pageLeft = Number(pageBox.lower_left_x ?? pageBox.lowerLeftX ?? 0);
  const pageBottom = Number(pageBox.lower_left_y ?? pageBox.lowerLeftY ?? 0);
  const pageWidth = Number(pageBox.width);
  const pageHeight = Number(pageBox.height);
  if (!Number.isFinite(pageWidth) || !Number.isFinite(pageHeight) || pageWidth <= 0 || pageHeight <= 0) {
    return null;
  }

  const left = ((Number(bounds.lowerLeftX ?? bounds.lower_left_x) - pageLeft) / pageWidth) * 100;
  const right = ((Number(bounds.upperRightX ?? bounds.upper_right_x) - pageLeft) / pageWidth) * 100;
  const top = 100 - ((Number(bounds.upperRightY ?? bounds.upper_right_y) - pageBottom) / pageHeight) * 100;
  const bottom = 100 - ((Number(bounds.lowerLeftY ?? bounds.lower_left_y) - pageBottom) / pageHeight) * 100;
  return {
    left: clampPercent(Math.min(left, right)),
    top: clampPercent(Math.min(top, bottom)),
    width: Math.max(1, clampPercent(Math.abs(right - left))),
    height: Math.max(1, clampPercent(Math.abs(bottom - top))),
  };
}

function clampPercent(value) {
  if (!Number.isFinite(value)) {
    return 0;
  }
  return Math.min(100, Math.max(0, value));
}

function clampNumber(value, min, max) {
  if (!Number.isFinite(value)) {
    return min;
  }
  return Math.min(max, Math.max(min, value));
}

function pageObjectKindLabel(kind) {
  if (!kind) {
    return "Object";
  }
  return String(kind)
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function pageObjectKindIcon(kind) {
  const normalized = String(kind ?? "").toLowerCase();
  if (normalized === "text") return "T";
  if (normalized === "path") return "P";
  if (normalized === "image") return "I";
  if (normalized === "form") return "F";
  if (normalized === "annotation") return "A";
  return "O";
}

function pageObjectMeta(object) {
  const pieces = [pageObjectKindLabel(object.kind)];
  if (object.bbox) {
    pieces.push(formatPageObjectBounds(object.bbox));
  } else {
    pieces.push("no bbox");
  }
  if (object.reference) {
    pieces.push(objectLabel(object.reference));
  }
  return pieces.join(" · ");
}

function formatPageObjectBounds(bounds) {
  if (!bounds) {
    return "-";
  }
  return `[${formatNumber(bounds.lowerLeftX ?? bounds.lower_left_x)}, ${formatNumber(bounds.lowerLeftY ?? bounds.lower_left_y)}, ${formatNumber(bounds.upperRightX ?? bounds.upper_right_x)}, ${formatNumber(bounds.upperRightY ?? bounds.upper_right_y)}] (${formatNumber(bounds.width)} x ${formatNumber(bounds.height)})`;
}

async function loadPagePreview(page, options = {}) {
  const pageNumber = page?.page_number ?? page?.pageNumber;
  if (!currentPdfPath || !Number.isFinite(Number(pageNumber))) {
    clearPagePreviewImage("Select a page to render a PDFium preview.");
    return;
  }

  const renderZoom = normalizedPreviewZoom(options.zoom ?? pagePreviewZoom);
  const draftRevision = hasDocumentDraftEdits() ? objectEditDraftRevision : 0;
  const documentKey = previewDocumentCacheKey(currentPdfPath, draftRevision);
  const previewKey = previewCacheKey(documentKey, pageNumber, renderZoom);
  const cachedPreview = pagePreviewCache.get(previewKey);
  if (cachedPreview) {
    pagePreviewRenderZoom = renderZoom;
    const requestId = options.requestId ?? ++pagePreviewRequestId;
    const selectedPageNumber = activePage?.page_number ?? activePage?.pageNumber;
    if (
      requestId === pagePreviewRequestId &&
      Number(selectedPageNumber) === Number(pageNumber) &&
      samePreviewZoom(renderZoom, pagePreviewZoom)
    ) {
      setPagePreviewStatus(
        "cache-hit",
        `Cache hit for page ${pageNumber} at ${Math.round(renderZoom * 100)}%; loading cached image.`,
        `Preview cache hit page ${pageNumber}`,
      );
      if (draftRevision > 0) {
        updatePagePreviewDraftStatus(
          `Showing cached draft revision ${draftRevision} from a temporary local PDF. Save writes the current PDF; Save As writes a separate copy.`,
          { snapshot: true },
        );
      }
      renderPagePreviewImage(cachedPreview, options.anchor ?? pagePreviewRenderAnchor ?? lastPagePreviewZoomAnchor);
    }
    return;
  }

  pagePreviewRenderZoom = renderZoom;
  const hasDraftPreview = draftRevision > 0;
  const loadingMessage = hasDraftPreview
    ? `Rendering unsaved draft for page ${pageNumber} at ${Math.round(renderZoom * 100)}% with PDFium.`
    : `Rendering page ${pageNumber} at ${Math.round(renderZoom * 100)}% with PDFium.`;
  const preserveCurrent = Boolean(options.preserveCurrent && activePagePreview && !elements.pagePreviewFigure.hidden);
  if (preserveCurrent) {
    setPagePreviewStatus("rendering", loadingMessage, `Rendering page ${pageNumber}`);
    elements.pagePreviewError.hidden = true;
    elements.pagePreviewError.textContent = "";
  } else {
    setPagePreviewPending(loadingMessage, "rendering");
  }
  const requestId = options.requestId ?? ++pagePreviewRequestId;
  const perf = perfMark("render_page_preview", `page=${pageNumber} zoom=${renderZoom}`);

  try {
    let renderPath = currentPdfPath;
    if (hasDraftPreview) {
      const snapshot = await getDraftPreviewSnapshot();
      const selectedPageNumber = activePage?.page_number ?? activePage?.pageNumber;
      if (
        requestId !== pagePreviewRequestId ||
        Number(selectedPageNumber) !== Number(pageNumber) ||
        !samePreviewZoom(renderZoom, pagePreviewZoom) ||
        draftRevision !== objectEditDraftRevision ||
        !snapshot?.path
      ) {
        perfDone(perf, "stale-draft");
        setPagePreviewStatus(
          "stale",
          `Stale draft page ${pageNumber} render ignored because a newer selection or edit is active.`,
          "Stale draft preview ignored",
        );
        return;
      }
      renderPath = snapshot.path;
      updatePagePreviewDraftStatus(
        `Rendering draft revision ${draftRevision} from a temporary local PDF. Save writes the current PDF; Save As writes a separate copy.`,
        { snapshot: true },
      );
    }
    const preview = await loadPagePreviewFromCacheOrTauri(
      renderPath,
      documentKey,
      pageNumber,
      renderZoom,
      requestId,
    );
    const selectedPageNumber = activePage?.page_number ?? activePage?.pageNumber;
    if (
      requestId !== pagePreviewRequestId ||
      Number(selectedPageNumber) !== Number(pageNumber) ||
      !samePreviewZoom(renderZoom, pagePreviewZoom) ||
        draftRevision !== (hasDocumentDraftEdits() ? objectEditDraftRevision : 0)
    ) {
      perfDone(perf, "stale");
      setPagePreviewStatus(
        "stale",
        `Stale page ${pageNumber} render ignored because a newer selection is active.`,
        "Stale preview ignored",
      );
      return;
    }
    perfDone(perf);
    pagePreviewCache.set(previewKey, preview);
    if (hasDraftPreview) {
      updatePagePreviewDraftStatus(
        `Showing draft revision ${draftRevision} from a temporary local PDF. Save writes the current PDF; Save As writes a separate copy.`,
        { snapshot: true },
      );
    }
    renderPagePreviewImage(preview, options.anchor ?? pagePreviewRenderAnchor ?? lastPagePreviewZoomAnchor);
  } catch (error) {
    if (requestId !== pagePreviewRequestId) {
      perfDone(perf, "stale-error");
      setPagePreviewStatus(
        "stale",
        `Stale page ${pageNumber} render error ignored because a newer selection is active.`,
        "Stale preview ignored",
      );
      return;
    }
    perfDone(perf, "error");
    if (hasDraftPreview) {
      updatePagePreviewDraftStatus(
        `Draft preview could not be rendered: ${String(error)} Save As is still available for supported small PDFs.`,
      );
    }
    showPagePreviewError(String(error), { preserveCurrent });
  }
}

async function loadPagePreviewFromCacheOrTauri(path, documentKey, pageNumber, zoom, requestId) {
  const key = previewCacheKey(documentKey, pageNumber, zoom);
  const cachedPreview = pagePreviewCache.get(key);
  if (cachedPreview) {
    return cachedPreview;
  }
  const pendingPreview = pagePreviewLoading.get(key);
  if (pendingPreview) {
    return pendingPreview;
  }
  let request = null;
  request = scheduleTauriTask(
    "pagePreview",
    "render_page_preview",
    {
      path,
      pageNumber: Number(pageNumber),
      zoom,
      requestId: Number(requestId),
      openGeneration: Number(pdfOpenGeneration),
    },
    {
      dropGroup: "pagePreview",
      coalesceKey: `pagePreview:${documentKey}:${Number(pageNumber)}:${Math.round(normalizedPreviewZoom(zoom) * 1000)}`,
    },
  )
    .then((preview) => {
      if (documentKey === previewDocumentCacheKey(currentPdfPath, hasDocumentDraftEdits() ? objectEditDraftRevision : 0)) {
        pagePreviewCache.set(key, preview);
      }
      return preview;
    })
    .finally(() => {
      if (pagePreviewLoading.get(key) === request) {
        pagePreviewLoading.delete(key);
      }
    });
  pagePreviewLoading.set(key, request);
  return request;
}

function renderPagePreviewImage(preview, anchor = null) {
  elements.pagePreviewError.hidden = true;
  elements.pagePreviewError.textContent = "";
  const assetUrl = convertFileSrc(preview.path);
  const renderedZoom = normalizedPreviewZoom(preview.zoom ?? pagePreviewZoom);
  const pendingKey = `${assetUrl}:${renderedZoom}`;
  elements.pagePreviewImage.dataset.pendingPreview = pendingKey;
  setPagePreviewStatus(
    "loading",
    `PDFium render complete for page ${preview.pageNumber}; loading preview image at ${Math.round(renderedZoom * 100)}%.`,
    `Loading page ${preview.pageNumber} image`,
  );
  const loader = new Image();
  loader.onload = () => {
    if (
      elements.pagePreviewImage.dataset.pendingPreview !== pendingKey ||
      !samePreviewZoom(renderedZoom, pagePreviewZoom)
    ) {
      return;
    }
    const commit = () => {
      if (
        elements.pagePreviewImage.dataset.pendingPreview !== pendingKey ||
        !samePreviewZoom(renderedZoom, pagePreviewZoom)
      ) {
        return;
      }
      delete elements.pagePreviewImage.dataset.pendingPreview;
      withInstantPagePreviewSwap(() => {
        activePagePreview = preview;
        showPagePreviewBitmapState();
        elements.pagePreviewImage.src = assetUrl;
        elements.pagePreviewImage.dataset.previewSrc = assetUrl;
        elements.pagePreviewImage.dataset.previewZoom = String(renderedZoom);
        restartElementTransition(elements.pagePreviewImage);
        elements.pagePreviewCaption.textContent = `${preview.pixelWidth} x ${preview.pixelHeight} px`;
        elements.pagePreviewFigure.hidden = false;
        setPagePreviewStatus(
          "complete",
          `Complete: page ${preview.pageNumber} rendered at ${Math.round(renderedZoom * 100)}% with ${preview.renderer}.`,
          `Preview complete page ${preview.pageNumber}`,
        );
        pagePreviewRenderZoom = null;
        pagePreviewRenderAnchor = null;
        applyPagePreviewZoom(anchor);
      });
      updatePagePreviewZoomControls();
      if (selectedPageObject) {
        showPageObjectOverlay(selectedPageObject);
      }
    };

    commit();
  };
  loader.onerror = () => {
    if (
      elements.pagePreviewImage.dataset.pendingPreview !== pendingKey ||
      !samePreviewZoom(renderedZoom, pagePreviewZoom)
    ) {
      return;
    }
    delete elements.pagePreviewImage.dataset.pendingPreview;
    showPagePreviewError(
      `Rendered preview was written to ${preview.path}, but the image asset could not be loaded from ${assetUrl}.`,
      { preserveCurrent: Boolean(activePagePreview && !elements.pagePreviewFigure.hidden) },
    );
  };
  loader.src = assetUrl;
}

function showPagePreviewError(message, options = {}) {
  setPagePreviewStatus("failed", "Failed: page preview is unavailable.", "Preview failed");
  elements.pagePreviewError.textContent = pagePreviewErrorMessage(message);
  elements.pagePreviewError.hidden = false;
  if (options.preserveCurrent && activePagePreview && !elements.pagePreviewFigure.hidden) {
    setPagePreviewStatus(
      "failed",
      "Failed: page preview re-render failed; showing the last completed preview.",
      `Showing rendered page ${activePagePreview.pageNumber ?? activePagePreview.page_number}`,
    );
    updatePagePreviewZoomControls();
    return;
  }

  activePagePreview = null;
  pagePreviewRenderZoom = null;
  elements.pagePreviewImage.onload = null;
  elements.pagePreviewImage.onerror = null;
  delete elements.pagePreviewImage.dataset.pendingPreview;
  delete elements.pagePreviewImage.dataset.previewSrc;
  delete elements.pagePreviewImage.dataset.previewZoom;
  elements.pagePreviewImage.removeAttribute("src");
  elements.pagePreviewImage.hidden = true;
  elements.pagePreviewCaption.textContent = "";
  clearPagePreviewStageLayout();
  updatePagePreviewZoomControls();
  clearPageObjectOverlay();
  showPagePreviewEmptyState("Preview unavailable", "The rendered page image could not be displayed.");
}

function clearPagePreviewImage(message) {
  cancelScheduledPagePreviewRender();
  pagePreviewRequestId += 1;
  pagePreviewRenderZoom = null;
  setPagePreviewStatus("idle", message);
  elements.pagePreviewError.hidden = true;
  elements.pagePreviewError.textContent = "";
  activePagePreview = null;
  elements.pagePreviewImage.onload = null;
  elements.pagePreviewImage.onerror = null;
  delete elements.pagePreviewImage.dataset.pendingPreview;
  delete elements.pagePreviewImage.dataset.previewSrc;
  delete elements.pagePreviewImage.dataset.previewZoom;
  elements.pagePreviewImage.removeAttribute("src");
  elements.pagePreviewImage.hidden = true;
  elements.pagePreviewCaption.textContent = "";
  clearPagePreviewStageLayout();
  updatePagePreviewZoomControls();
  clearPageObjectOverlay();
  showPagePreviewEmptyState(previewEmptyTitleForMessage(message), message);
}

function setPagePreviewPending(message, state = "loading") {
  pagePreviewRenderZoom = null;
  setPagePreviewStatus(state, message);
  elements.pagePreviewError.hidden = true;
  elements.pagePreviewError.textContent = "";
  activePagePreview = null;
  elements.pagePreviewImage.onload = null;
  elements.pagePreviewImage.onerror = null;
  delete elements.pagePreviewImage.dataset.pendingPreview;
  delete elements.pagePreviewImage.dataset.previewSrc;
  delete elements.pagePreviewImage.dataset.previewZoom;
  elements.pagePreviewImage.removeAttribute("src");
  elements.pagePreviewImage.hidden = true;
  elements.pagePreviewCaption.textContent = "";
  clearPagePreviewStageLayout();
  updatePagePreviewZoomControls();
  clearPageObjectOverlay();
  showPagePreviewEmptyState(previewEmptyTitleForMessage(message), message);
}

function showPagePreviewEmptyState(title, message) {
  elements.pagePreviewFigure.hidden = false;
  elements.pagePreviewEmpty.hidden = false;
  elements.pagePreviewEmptyTitle.textContent = title;
  elements.pagePreviewEmptyMessage.textContent = message;
  elements.pagePreviewCanvas.hidden = true;
  elements.pagePreviewImage.hidden = true;
  elements.pagePreviewCaption.textContent = "";
}

function showPagePreviewBitmapState() {
  elements.pagePreviewFigure.hidden = false;
  elements.pagePreviewEmpty.hidden = true;
  elements.pagePreviewCanvas.hidden = false;
  elements.pagePreviewImage.hidden = false;
}

function previewEmptyTitleForMessage(message) {
  const normalized = String(message ?? "").toLowerCase();
  if (normalized.includes("rendering") || normalized.includes("loading")) {
    return "Rendering preview";
  }
  if (normalized.includes("unavailable") || normalized.includes("failed") || normalized.includes("could not")) {
    return "Preview unavailable";
  }
  if (normalized.includes("select")) {
    return "No page selected";
  }
  return "No page rendered";
}

function pagePreviewErrorMessage(message) {
  return `${message} If this is a PDFium runtime issue, run npm run pdfium:prepare or set PDF_DEBUGGER_PDFIUM_PATH.`;
}

function pageDimensions(page) {
  const box = page?.media_box ?? page?.mediaBox;
  if (!box) {
    return "Unknown size";
  }
  return `${formatNumber(box.width)} x ${formatNumber(box.height)}`;
}

function formatPageBox(box) {
  if (!box) {
    return "-";
  }
  return `[${formatNumber(box.lower_left_x ?? box.lowerLeftX)}, ${formatNumber(box.lower_left_y ?? box.lowerLeftY)}, ${formatNumber(box.upper_right_x ?? box.upperRightX)}, ${formatNumber(box.upper_right_y ?? box.upperRightY)}] (${formatNumber(box.width)} x ${formatNumber(box.height)})`;
}

function formatNumber(value) {
  if (!Number.isFinite(Number(value))) {
    return "-";
  }
  const number = Number(value);
  return Number.isInteger(number) ? number.toString() : number.toFixed(2);
}

function formatByteRange(range) {
  const normalized = normalizedRange(range);
  return normalized ? `${normalized.start}..${normalized.end}` : "-";
}

function renderTrailer(trailer, fileName = null) {
  activeTrailer = trailer;
  const fragment = document.createDocumentFragment();
  elements.trailerScroll.onscroll = null;
  virtualTrailerTreeState = null;
  elements.trailerError.hidden = true;
  elements.trailerError.textContent = "";

  if (!trailer) {
    elements.trailerSubtitle.textContent = "Open a PDF to inspect the trailer dictionary";
    elements.trailerState.textContent = "Idle";
    fragment.appendChild(emptyTrailerRow("Open a PDF to populate trailer entries."));
    elements.trailerEntries.replaceChildren(fragment);
    return;
  }

  const nodes = Array.isArray(trailer.nodes) ? trailer.nodes : [];
  const entries = Array.isArray(trailer.entries) ? trailer.entries : [];
  const rootNodes = nodes.length ? nodes : entries.map(trailerEntryToNode);
  const warnings = Array.isArray(trailer.warnings) ? trailer.warnings : [];
  elements.trailerSubtitle.textContent = fileName
    ? `Trailer dictionary for ${fileName}`
    : "Trailer dictionary";
  elements.trailerState.textContent = rootNodes.length ? "Ready" : "Empty";

  if (warnings.length) {
    elements.trailerError.textContent = warnings.join(" ");
    elements.trailerError.hidden = false;
  }

  if (!rootNodes.length) {
    fragment.appendChild(emptyTrailerRow("No trailer dictionary entries are available."));
    elements.trailerEntries.replaceChildren(fragment);
    return;
  }

  const treeContext = trailerTreeContext();
  const state = { rows: 0, limitHit: false, rowsList: [], tree: treeContext };
  for (const node of rootNodes) {
    appendStructureNodeRow(node, {
      depth: 0,
      path: trailerNodeKey(node, "root"),
      ancestors: new Set(),
      state,
    });
  }
  if (state.limitHit) {
    state.rowsList.push({
      kind: "empty",
      message: `Trailer tree row limit reached (${TRAILER_TREE_MAX_ROWS}). Collapse nodes or inspect objects directly.`,
    });
  }
  renderVirtualTrailerRows(state.rowsList);
  elements.trailerScroll.onscroll = () => scheduleVirtualRender(virtualTrailerTreeState);
}

function emptyTrailerRow(message) {
  const row = document.createElement("tr");
  const cell = document.createElement("td");
  cell.className = "empty-table-cell";
  cell.colSpan = 3;
  cell.textContent = message;
  row.appendChild(cell);
  return row;
}

function renderVirtualTrailerRows(rowsList) {
  const rows = Array.isArray(rowsList) ? rowsList : [];
  const range = virtualRange(elements.trailerScroll, rows.length, TRAILER_TREE_ROW_HEIGHT);
  const fragment = document.createDocumentFragment();
  if (range.enabled && range.before) {
    fragment.appendChild(trailerSpacerRow(range.before));
  }
  for (let index = range.start; index < range.end; index += 1) {
    fragment.appendChild(renderTrailerVirtualRow(rows[index]));
  }
  if (range.enabled && range.after) {
    fragment.appendChild(trailerSpacerRow(range.after));
  }
  elements.trailerEntries.replaceChildren(fragment);
  virtualTrailerTreeState = {
    frame: null,
    rows,
    render: () => renderVirtualTrailerRows(rows),
  };
}

function renderObjectDetailsTree(node, options = {}) {
  activeObjectDetailsNode = node ?? null;
  elements.objectDetailsScroll.onscroll = null;
  virtualObjectDetailsTreeState = null;

  if (!activeObjectDetailsNode) {
    const fragment = document.createDocumentFragment();
    fragment.appendChild(emptyTrailerRow("Select an object to inspect its structure."));
    elements.objectDetailsEntries.replaceChildren(fragment);
    return;
  }

  const treeContext = objectDetailsTreeContext();
  const rootPath = `object-root:${trailerNodeKey(activeObjectDetailsNode, "object")}`;
  if (options.expandRoot) {
    treeContext.expandedKeys.add(rootPath);
  }
  const state = { rows: 0, limitHit: false, rowsList: [], tree: treeContext };
  appendStructureNodeRow(activeObjectDetailsNode, {
    depth: 0,
    path: rootPath,
    editPath: [],
    ancestors: new Set(),
    state,
  });
  if (state.limitHit) {
    state.rowsList.push({
      kind: "empty",
      message: `Object details row limit reached (${TRAILER_TREE_MAX_ROWS}). Collapse nodes or inspect referenced objects directly.`,
    });
  }
  renderVirtualObjectDetailsRows(state.rowsList);
  elements.objectDetailsScroll.onscroll = () => scheduleVirtualRender(virtualObjectDetailsTreeState);
}

function renderVirtualObjectDetailsRows(rowsList) {
  const rows = Array.isArray(rowsList) ? rowsList : [];
  const range = virtualRange(elements.objectDetailsScroll, rows.length, TRAILER_TREE_ROW_HEIGHT);
  const fragment = document.createDocumentFragment();
  if (range.enabled && range.before) {
    fragment.appendChild(trailerSpacerRow(range.before));
  }
  for (let index = range.start; index < range.end; index += 1) {
    fragment.appendChild(renderTrailerVirtualRow(rows[index]));
  }
  if (range.enabled && range.after) {
    fragment.appendChild(trailerSpacerRow(range.after));
  }
  elements.objectDetailsEntries.replaceChildren(fragment);
  virtualObjectDetailsTreeState = {
    frame: null,
    rows,
    render: () => renderVirtualObjectDetailsRows(rows),
  };
}

function renderTrailerVirtualRow(rowInfo) {
  if (rowInfo.kind === "node") {
    return renderStructureNodeRow(rowInfo.node, rowInfo.options);
  }
  if (rowInfo.kind === "empty") {
    return emptyTrailerRow(rowInfo.message);
  }
  return rowInfo.element;
}

function trailerSpacerRow(height) {
  const row = document.createElement("tr");
  row.className = "virtual-spacer-row";
  const cell = document.createElement("td");
  cell.colSpan = 3;
  cell.style.height = `${Math.max(0, Math.round(height))}px`;
  cell.setAttribute("aria-hidden", "true");
  row.appendChild(cell);
  return row;
}

function resetTrailerTreeState() {
  trailerExpandedKeys = new Set();
  trailerVisibleChildCounts = new Map();
  trailerLoadedObjects = new Map();
  trailerLoadingObjects = new Set();
  trailerLoadErrors = new Map();
  virtualTrailerTreeState = null;
  if (elements.trailerScroll) {
    elements.trailerScroll.onscroll = null;
  }
}

function resetObjectDetailsTreeState() {
  activeObjectDetailsNode = null;
  objectDetailsExpandedKeys = new Set();
  objectDetailsVisibleChildCounts = new Map();
  objectDetailsLoadedObjects = new Map();
  objectDetailsLoadingObjects = new Set();
  objectDetailsLoadErrors = new Map();
  virtualObjectDetailsTreeState = null;
  if (elements.objectDetailsScroll) {
    elements.objectDetailsScroll.onscroll = null;
  }
}

function trailerTreeContext() {
  return {
    id: "trailer",
    active: () => activeTrailer,
    render: () => renderTrailer(activeTrailer, currentPdfPath ? fileNameFromPath(currentPdfPath) : null),
    expandedKeys: trailerExpandedKeys,
    visibleChildCounts: trailerVisibleChildCounts,
    loadedObjects: trailerLoadedObjects,
    loadingObjects: trailerLoadingObjects,
    loadErrors: trailerLoadErrors,
    batchSize: TRAILER_TREE_CHILD_BATCH,
    loadTaskKind: "trailerObject",
    loadTaskDropGroup: "trailerObject",
    loadPerfName: "load_trailer_object",
  };
}

function objectDetailsTreeContext() {
  return {
    id: "objectDetails",
    active: () => activeObjectDetailsNode,
    render: () => renderObjectDetailsTree(activeObjectDetailsNode),
    expandedKeys: objectDetailsExpandedKeys,
    visibleChildCounts: objectDetailsVisibleChildCounts,
    loadedObjects: objectDetailsLoadedObjects,
    loadingObjects: objectDetailsLoadingObjects,
    loadErrors: objectDetailsLoadErrors,
    batchSize: TRAILER_TREE_CHILD_BATCH,
    loadTaskKind: "objectDetailsReference",
    loadTaskDropGroup: "objectDetailsReference",
    loadPerfName: "load_object_detail_reference",
  };
}

function renderAcroForm(view, options = {}) {
  if (view !== undefined) {
    activeAcroForm = view;
  }

  const fileName = options.fileName ?? (currentPdfPath ? fileNameFromPath(currentPdfPath) : null);
  const isIdle = Boolean(options.idle);
  const fields = Array.isArray(activeAcroForm?.fields) ? activeAcroForm.fields : [];
  const warnings = Array.isArray(activeAcroForm?.warnings) ? activeAcroForm.warnings : [];
  const query = acroFormSearchQuery.trim().toLowerCase();
  const filteredFields = query
    ? fields.filter((field) => String(field.name ?? "").toLowerCase().includes(query))
    : fields;

  elements.acroformSubtitle.textContent = fileName
    ? `Read-only AcroForm fields for ${fileName}`
    : "Open a PDF to inspect form fields";
  elements.acroformState.classList.remove("is-loading");
  elements.acroformState.textContent = isIdle
    ? "Idle"
    : fields.length
      ? "Ready"
      : currentPdfPath
        ? "Empty"
        : "Idle";
  elements.acroformError.hidden = true;
  elements.acroformError.textContent = "";
  elements.acroformCount.textContent = query
    ? `${filteredFields.length}/${fields.length}`
    : fields.length.toString();

  if (warnings.length) {
    elements.acroformWarnings.hidden = false;
    elements.acroformWarnings.textContent = warnings.join(" ");
  } else {
    elements.acroformWarnings.hidden = true;
    elements.acroformWarnings.textContent = "";
  }

  elements.acroformList.replaceChildren();
  if (!currentPdfPath) {
    elements.acroformList.classList.add("empty-list");
    elements.acroformList.textContent = "Open a PDF to populate AcroForm fields.";
    return;
  }
  if (isIdle && !activeAcroForm) {
    elements.acroformList.classList.add("empty-list");
    elements.acroformList.textContent = "Open the AcroForm tab to load form fields.";
    return;
  }
  if (!fields.length) {
    elements.acroformList.classList.add("empty-list");
    elements.acroformList.textContent = "No AcroForm fields found.";
    return;
  }
  if (!filteredFields.length) {
    elements.acroformList.classList.add("empty-list");
    elements.acroformList.textContent = "No fields match the current search.";
    return;
  }

  elements.acroformList.classList.remove("empty-list");
  const fragment = document.createDocumentFragment();
  for (const field of filteredFields) {
    fragment.appendChild(renderAcroFormFieldRow(field));
  }
  elements.acroformList.appendChild(fragment);
}

function renderAcroFormFieldRow(field) {
  const row = document.createElement("div");
  row.className = "acroform-field-row";

  const main = document.createElement("div");
  main.className = "acroform-field-main";

  const name = document.createElement("div");
  name.className = "acroform-field-name";
  name.textContent = field.name || "<unnamed>";
  name.title = field.name || "<unnamed>";
  main.appendChild(name);

  const meta = document.createElement("div");
  meta.className = "acroform-field-meta";
  meta.textContent = field.fieldType ? `/${field.fieldType}` : "Field";
  main.appendChild(meta);

  const reference = normalizeReference(field.reference);
  const button = document.createElement("button");
  button.className = "reference-chip acroform-reference";
  button.type = "button";
  button.textContent = objectLabel(reference);
  button.title = `Inspect ${objectLabel(reference)}`;
  button.addEventListener("click", () => openObjectWorkspaceTab(reference));

  row.appendChild(main);
  row.appendChild(button);
  return row;
}

function renderAnnotations(view, options = {}) {
  if (view !== undefined) {
    activeAnnotations = view;
  }

  const fileName = options.fileName ?? (currentPdfPath ? fileNameFromPath(currentPdfPath) : null);
  const annotations = Array.isArray(activeAnnotations?.annotations) ? activeAnnotations.annotations : [];
  const warnings = Array.isArray(activeAnnotations?.warnings) ? activeAnnotations.warnings : [];
  const query = annotsSearchQuery.trim().toLowerCase();
  const filteredAnnotations = query
    ? annotations.filter((annotation) => annotationSearchText(annotation).includes(query))
    : annotations;

  elements.annotsSubtitle.textContent = fileName
    ? `Read-only annotations for ${fileName}`
    : "Open a PDF to inspect annotations";
  elements.annotsState.classList.remove("is-loading");
  elements.annotsState.textContent = annotations.length
    ? "Ready"
    : currentPdfPath
      ? "Empty"
      : "Idle";
  elements.annotsError.hidden = true;
  elements.annotsError.textContent = "";
  elements.annotsCount.textContent = query
    ? `${filteredAnnotations.length}/${annotations.length}`
    : annotations.length.toString();

  if (warnings.length) {
    elements.annotsWarnings.hidden = false;
    elements.annotsWarnings.textContent = warnings.join(" ");
  } else {
    elements.annotsWarnings.hidden = true;
    elements.annotsWarnings.textContent = "";
  }

  elements.annotsList.replaceChildren();
  if (!currentPdfPath) {
    elements.annotsList.classList.add("empty-list");
    elements.annotsList.textContent = "Open a PDF to populate annotations.";
    return;
  }
  if (!annotations.length) {
    elements.annotsList.classList.add("empty-list");
    elements.annotsList.textContent = "No annotations found.";
    return;
  }
  if (!filteredAnnotations.length) {
    elements.annotsList.classList.add("empty-list");
    elements.annotsList.textContent = "No annotations match the current search.";
    return;
  }

  elements.annotsList.classList.remove("empty-list");
  const fragment = document.createDocumentFragment();
  for (const annotation of filteredAnnotations) {
    fragment.appendChild(renderAnnotationRow(annotation));
  }
  elements.annotsList.appendChild(fragment);
}

function annotationSearchText(annotation) {
  const reference = annotation.reference ? normalizeReference(annotation.reference) : null;
  const pageReferenceValue = annotation.pageReference ?? annotation.page_reference;
  const pageReference = pageReferenceValue ? normalizeReference(pageReferenceValue) : null;
  return [
    annotation.pageNumber ?? annotation.page_number,
    objectLabel(reference),
    objectLabel(pageReference),
    annotation.subtype,
    annotation.rect,
    annotation.flags,
    annotation.contents,
    annotation.color,
    annotation.border,
    annotation.appearance,
    annotation.ca,
    ...(Array.isArray(annotation.keys) ? annotation.keys : []),
  ]
    .filter((value) => value !== null && value !== undefined)
    .map((value) => String(value).toLowerCase())
    .join(" ");
}

function renderAnnotationRow(annotation) {
  const row = document.createElement("article");
  row.className = "annotation-row";
  row.tabIndex = 0;

  const reference = annotation.reference ? normalizeReference(annotation.reference) : null;
  const pageNumber = Number(annotation.pageNumber ?? annotation.page_number);
  const subtype = annotation.subtype || "/Annot";
  row.title = reference ? `Inspect annotation ${objectLabel(reference)}` : "Inspect annotation";
  row.addEventListener("click", () => {
    if (reference) {
      openObjectWorkspaceTab(reference);
    }
  });
  row.addEventListener("keydown", (event) => {
    if ((event.key === "Enter" || event.key === " ") && reference) {
      event.preventDefault();
      openObjectWorkspaceTab(reference);
    }
  });

  const header = document.createElement("div");
  header.className = "annotation-row-header";

  const title = document.createElement("div");
  title.className = "annotation-title";
  title.textContent = subtype;
  title.title = subtype;
  header.appendChild(title);

  const actions = document.createElement("div");
  actions.className = "annotation-actions";
  if (Number.isFinite(pageNumber)) {
    const pageButton = document.createElement("button");
    pageButton.className = "secondary-button compact-button annotation-page-button";
    pageButton.type = "button";
    pageButton.textContent = `Page ${pageNumber}`;
    pageButton.title = `Open page ${pageNumber}`;
    pageButton.addEventListener("click", (event) => {
      event.stopPropagation();
      openAnnotationPage(pageNumber);
    });
    actions.appendChild(pageButton);
  }
  if (reference) {
    const refButton = document.createElement("button");
    refButton.className = "reference-chip annotation-reference";
    refButton.type = "button";
    refButton.textContent = objectLabel(reference);
    refButton.title = `Inspect ${objectLabel(reference)}`;
    refButton.addEventListener("click", (event) => {
      event.stopPropagation();
      openObjectWorkspaceTab(reference);
    });
    actions.appendChild(refButton);
  }
  header.appendChild(actions);
  row.appendChild(header);

  const fields = document.createElement("div");
  fields.className = "annotation-fields";
  appendAnnotationField(fields, "Rect", annotation.rect);
  appendAnnotationField(fields, "F", annotation.flags);
  appendAnnotationField(fields, "Contents", annotation.contents);
  appendAnnotationField(fields, "C", annotation.color);
  appendAnnotationField(fields, "Border", annotation.border);
  appendAnnotationField(fields, "AP", annotation.appearance);
  appendAnnotationField(fields, "CA", annotation.ca);
  row.appendChild(fields);

  return row;
}

function appendAnnotationField(container, label, value) {
  const normalized = String(value ?? "").trim();
  if (!normalized) {
    return;
  }
  const field = document.createElement("div");
  field.className = "annotation-field";

  const name = document.createElement("span");
  name.className = "annotation-field-name";
  name.textContent = `/${label}`;
  field.appendChild(name);

  const text = document.createElement("span");
  text.className = "annotation-field-value";
  text.textContent = normalized;
  text.title = normalized;
  field.appendChild(text);

  container.appendChild(field);
}

function openAnnotationPage(pageNumber) {
  const normalized = Number(pageNumber);
  if (!Number.isFinite(normalized)) {
    setActiveWorkspaceTab("page");
    return;
  }
  const page = activePages.find((item) => Number(item.page_number ?? item.pageNumber) === normalized);
  if (!page) {
    setActiveWorkspaceTab("page");
    return;
  }
  openPageWorkspaceTab(page, pageButtonForPage(page));
}

async function ensureAcroFormLoaded() {
  if (!currentPdfPath) {
    clearAcroForm();
    return;
  }
  if (activeAcroForm) {
    renderAcroForm(activeAcroForm);
    return;
  }

  const requestPath = currentPdfPath;
  const requestId = ++acroFormRequestId;
  elements.acroformState.textContent = "Loading";
  elements.acroformState.classList.add("is-loading");
  elements.acroformError.hidden = true;
  elements.acroformError.textContent = "";
  elements.acroformWarnings.hidden = true;
  elements.acroformWarnings.textContent = "";
  elements.acroformList.classList.add("empty-list");
  elements.acroformList.textContent = "Loading AcroForm fields...";

  const perf = perfMark("load_acroform", fileNameFromPath(requestPath));
  try {
    const view = await loadAcroFormFromCacheOrTauri(requestPath);
    if (requestId !== acroFormRequestId || requestPath !== currentPdfPath) {
      perfDone(perf, "stale");
      return;
    }
    perfDone(perf);
    activeAcroForm = view;
    renderAcroForm(view);
  } catch (error) {
    if (requestId !== acroFormRequestId || requestPath !== currentPdfPath) {
      perfDone(perf, "stale-error");
      return;
    }
    perfDone(perf, "error");
    elements.acroformState.textContent = "Error";
    elements.acroformError.textContent = String(error);
    elements.acroformError.hidden = false;
    elements.acroformList.classList.add("empty-list");
    elements.acroformList.textContent = "AcroForm fields could not be loaded.";
  } finally {
    if (requestId === acroFormRequestId) {
      elements.acroformState.classList.remove("is-loading");
    }
  }
}

async function loadAcroFormFromCacheOrTauri(path) {
  const cached = acroFormCache.get(path);
  if (cached) {
    return cached;
  }
  const pending = acroFormLoading.get(path);
  if (pending) {
    return pending;
  }

  let request = null;
  request = scheduleTauriTask(
    "acroform",
    "load_acroform",
    { path },
    {
      dropGroup: "acroform",
      coalesceKey: `acroform:${path}`,
    },
  )
    .then((view) => {
      if (path === currentPdfPath) {
        acroFormCache.set(path, view);
      }
      return view;
    })
    .finally(() => {
      if (acroFormLoading.get(path) === request) {
        acroFormLoading.delete(path);
      }
    });
  acroFormLoading.set(path, request);
  return request;
}

function resetObjectInspectionCache() {
  objectInspectionCache = new Map();
  objectInspectionLoading = new Map();
  streamMetadataCache = new Map();
  streamMetadataLoading = new Map();
  streamViewCache = new Map();
  streamViewLoading = new Map();
  streamPreviewCache = new Map();
  streamPreviewLoading = new Map();
  streamImagePreviewCache = new Map();
  streamImagePreviewLoading = new Map();
  pagePreviewCache = new Map();
  pagePreviewLoading = new Map();
  acroFormCache = new Map();
  acroFormLoading = new Map();
  clearDraftPreviewSnapshots();
}

function objectDraftsToTauriEdits() {
  return Array.from(objectEditDrafts.values()).map((draft) => ({
    object: draft.reference.object,
    generation: draft.reference.generation,
    path: draft.path,
    value: draft.input,
  }));
}

function streamDraftsToTauriEdits() {
  return Array.from(streamEditDrafts.values()).map((draft) => ({
    object: draft.reference.object,
    generation: draft.reference.generation,
    decodedText: draft.decodedText,
  }));
}

function hasDocumentDraftEdits() {
  return objectEditDrafts.size > 0 || streamEditDrafts.size > 0;
}

function currentDraftPreviewKey() {
  if (!currentPdfPath || !hasDocumentDraftEdits() || objectEditDraftRevision <= 0) {
    return null;
  }
  return `${currentPdfPath}::draft:${objectEditDraftRevision}`;
}

function clearDraftPreviewSnapshots() {
  draftPreviewSnapshots = new Map();
  draftPreviewSnapshotLoading = new Map();
}

function markObjectDraftsChanged() {
  objectEditDraftRevision += 1;
  clearDraftPreviewSnapshots();
  pagePreviewCache = new Map();
  pagePreviewLoading = new Map();
  clearPagePreviewImage(
    hasDocumentDraftEdits()
      ? "Unsaved document edits changed. Select the page to render a temporary draft preview."
      : "Document edit drafts were cleared. Select the page to render the current PDF.",
  );
  updatePagePreviewDraftStatus();
  if (activePage && currentPdfPath) {
    schedulePagePreviewRender(PAGE_PREVIEW_RENDER_DEBOUNCE_MS);
  }
}

function resetObjectEditDrafts() {
  objectEditDrafts = new Map();
  streamEditDrafts = new Map();
  activeStreamEditKey = null;
  activeObjectEditKey = null;
  activeObjectEditPathKey = null;
  objectEditDraftRevision += 1;
  clearDraftPreviewSnapshots();
  updateObjectEditToolbar();
  updatePagePreviewDraftStatus();
  updateStreamEditControls();
}

async function getDraftPreviewSnapshot() {
  const key = currentDraftPreviewKey();
  if (!key) {
    return null;
  }
  const cached = draftPreviewSnapshots.get(key);
  if (cached) {
    return cached;
  }
  const pending = draftPreviewSnapshotLoading.get(key);
  if (pending) {
    return pending;
  }

  const revision = objectEditDraftRevision;
  const path = currentPdfPath;
  const edits = objectDraftsToTauriEdits();
  const streamEdits = streamDraftsToTauriEdits();
  const editCount = edits.length + streamEdits.length;
  updatePagePreviewDraftStatus(
    `Creating temporary preview PDF for ${editCount} unsaved edit${editCount === 1 ? "" : "s"}...`,
  );
  let request = null;
  request = scheduleTauriTask(
    "pagePreview",
    "create_draft_preview_snapshot",
    {
      path,
      edits,
      streamEdits,
      revision,
    },
    {
      dropGroup: "pagePreview",
      coalesceKey: `draftPreview:${path}:${revision}`,
    },
  )
    .then((snapshot) => {
      if (path === currentPdfPath && revision === objectEditDraftRevision && hasDocumentDraftEdits()) {
        draftPreviewSnapshots.set(key, snapshot);
      }
      return snapshot;
    })
    .finally(() => {
      if (draftPreviewSnapshotLoading.get(key) === request) {
        draftPreviewSnapshotLoading.delete(key);
      }
    });
  draftPreviewSnapshotLoading.set(key, request);
  return request;
}

function activePageNumber() {
  return activePage?.page_number ?? activePage?.pageNumber ?? null;
}

async function selectPageNumberAfterLoad(pageNumber, options = {}) {
  const normalized = Number(pageNumber);
  if (!Number.isFinite(normalized) || normalized <= 0) {
    return false;
  }

  const attempts = options.attempts ?? 40;
  const delayMs = options.delayMs ?? 100;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    const page = activePages.find((item) => Number(item.page_number ?? item.pageNumber) === normalized);
    if (page) {
      openPageWorkspaceTab(page, pageButtonForPage(page));
      selectPage(page, pageButtonForPage(page), { force: true, activateTab: false });
      return true;
    }
    await new Promise((resolve) => window.setTimeout(resolve, delayMs));
  }
  return false;
}

function trailerEntryToNode(entry) {
  return {
    kind: "value",
    key: entry.key ?? "-",
    valueType: entry.valueType ?? entry.value_type ?? "raw",
    value: entry.value ?? "-",
    reference: entry.reference ?? null,
    expandable: Boolean(entry.reference),
    children: [],
    stream: false,
  };
}

function appendStructureNodeRow(node, context) {
  if (context.state.rows >= TRAILER_TREE_MAX_ROWS) {
    context.state.limitHit = true;
    return;
  }

  const tree = context.state.tree ?? trailerTreeContext();
  const normalized = normalizeTrailerNode(node);
  const rowKey = context.path;
  const reference = normalized.reference ? normalizeReference(normalized.reference) : null;
  const referenceKeyValue = reference ? referenceKey(reference) : null;
  const isReferenceValue = normalized.kind !== "object" && Boolean(reference);
  const objectNode = normalized.kind === "object";
  const children = Array.isArray(normalized.children) ? normalized.children : [];
  const hasInlineChildren = children.length > 0;
  const canExpand = hasInlineChildren || isReferenceValue || Boolean(normalized.expandable);
  const isExpanded = tree.expandedKeys.has(rowKey);
  const isLoadingReference = Boolean(
    isExpanded &&
      canExpand &&
      isReferenceValue &&
      referenceKeyValue &&
      !context.ancestors.has(referenceKeyValue) &&
      !tree.loadErrors.has(referenceKeyValue) &&
      !tree.loadedObjects.has(referenceKeyValue)
  );

  context.state.rowsList.push({
    kind: "node",
    node: normalized,
    options: {
      depth: context.depth,
      rowKey,
      canExpand,
      isExpanded,
      reference,
      isReferenceValue,
      isLoading: isLoadingReference,
      tree,
      editReference: context.editReference ?? null,
      editPath: context.editPath ?? null,
    },
  });
  context.state.rows += 1;

  if (!isExpanded || !canExpand) {
    return;
  }

  if (context.depth >= TRAILER_TREE_MAX_DEPTH) {
    appendTrailerMessageRow("Expansion depth limit reached.", context.depth + 1, "warning", context.state);
    return;
  }

  if (isReferenceValue && referenceKeyValue) {
    if (context.ancestors.has(referenceKeyValue)) {
      appendTrailerMessageRow(`Cycle detected at ${objectLabel(reference)}.`, context.depth + 1, "warning", context.state);
      return;
    }
    if (tree.loadingObjects.has(referenceKeyValue)) {
      return;
    }
    if (tree.loadErrors.has(referenceKeyValue)) {
      appendTrailerMessageRow(tree.loadErrors.get(referenceKeyValue), context.depth + 1, "error", context.state);
      return;
    }

    const loaded = tree.loadedObjects.get(referenceKeyValue);
    if (!loaded) {
      loadStructureReference(reference, tree);
      return;
    }

    const nextAncestors = new Set(context.ancestors);
    nextAncestors.add(referenceKeyValue);
    appendLoadedReferenceProperties(loaded, {
      depth: context.depth + 1,
      path: `${rowKey}>ref:${referenceKeyValue}`,
      editReference: reference,
      editPath: [],
      ancestors: nextAncestors,
      state: context.state,
    });
    return;
  }

  if (!hasInlineChildren) {
    appendTrailerMessageRow("No expandable children.", context.depth + 1, "empty", context.state);
    return;
  }

  const nextAncestors = new Set(context.ancestors);
  if (objectNode && referenceKeyValue) {
    nextAncestors.add(referenceKeyValue);
  }
  const batchKey = childBatchKey(tree.id, rowKey);
  const visibleChildren = Math.min(children.length, getChildBatchSize(tree.visibleChildCounts, batchKey, tree.batchSize));
  children.slice(0, visibleChildren).forEach((child, index) => {
    appendStructureNodeRow(child, {
      depth: context.depth + 1,
      path: `${rowKey}>${index}:${trailerNodeKey(child, "child")}`,
      editReference: context.editReference ?? null,
      editPath: childEditPath(context.editPath, child),
      ancestors: nextAncestors,
      state: context.state,
    });
  });
  if (visibleChildren < children.length) {
    appendTrailerShowMoreRow({
      rowKey,
      batchKey,
      depth: context.depth + 1,
      shown: visibleChildren,
      total: children.length,
      state: context.state,
    });
  }
}

function appendLoadedReferenceProperties(loadedNode, context) {
  const normalized = normalizeTrailerNode(loadedNode);
  const children = Array.isArray(normalized.children) ? normalized.children : [];
  const tree = context.state.tree ?? trailerTreeContext();

  if (!children.length) {
    appendStructureNodeRow(normalized, context);
    return;
  }

  const batchKey = childBatchKey(tree.id, context.path);
  const visibleChildren = Math.min(children.length, getChildBatchSize(tree.visibleChildCounts, batchKey, tree.batchSize));
  children.slice(0, visibleChildren).forEach((child, index) => {
    appendStructureNodeRow(child, {
      depth: context.depth,
      path: `${context.path}>${index}:${trailerNodeKey(child, "child")}`,
      editReference: context.editReference ?? null,
      editPath: childEditPath(context.editPath, child),
      ancestors: context.ancestors,
      state: context.state,
    });
  });
  if (visibleChildren < children.length) {
    appendTrailerShowMoreRow({
      rowKey: context.path,
      batchKey,
      depth: context.depth,
      shown: visibleChildren,
      total: children.length,
      state: context.state,
    });
  }
}

function childEditPath(parentPath, child) {
  if (!Array.isArray(parentPath)) {
    return null;
  }
  const key = String(child?.key ?? "");
  if (key.startsWith("/") && key.length > 1) {
    return [...parentPath, { kind: "dict", key: key.slice(1) }];
  }
  const arrayMatch = key.match(/^\[(\d+)\]$/);
  if (arrayMatch) {
    return [...parentPath, { kind: "array", index: Number(arrayMatch[1]) }];
  }
  return null;
}

function canEditObjectDetailsValue(options) {
  return Boolean(
    options?.tree?.id === "objectDetails" &&
    normalizeEditReference(options) &&
    Array.isArray(options.editPath) &&
    isEditableObjectValueType(options.node),
  );
}

function isEditableObjectValueType(node) {
  const type = String(node?.valueType ?? node?.value_type ?? "").toLowerCase();
  return type === "number" || type === "string" || type === "name";
}

function normalizeEditReference(options) {
  const reference = options?.editReference ?? activeReference;
  return reference ? normalizeReference(reference) : null;
}

function objectEditEntryKey(optionsOrEditPath) {
  const editPath = Array.isArray(optionsOrEditPath)
    ? optionsOrEditPath
    : optionsOrEditPath?.editPath;
  const editReference = Array.isArray(optionsOrEditPath)
    ? activeReference
    : normalizeEditReference(optionsOrEditPath);
  if (!editReference || !Array.isArray(editPath)) {
    return null;
  }
  return `${referenceKey(editReference)}::${JSON.stringify(editPath)}`;
}

function editableValueForNode(node) {
  const type = String(node.valueType ?? node.value_type ?? "").toLowerCase();
  const value = String(node.value ?? "");
  if (type === "name") {
    return value.startsWith("/") ? value : `/${value}`;
  }
  if (type === "string") {
    if (value.startsWith("(") || value.startsWith("<")) {
      return value;
    }
    return `(${value})`;
  }
  if (type === "boolean" || type === "number" || type === "null" || type === "indirect reference") {
    return value;
  }
  if (type === "array" || type === "dictionary" || type === "raw") {
    return value;
  }
  return value;
}

function normalizedValidatedEditValue(validated) {
  const valueType = String(validated?.valueType ?? validated?.value_type ?? "").toLowerCase();
  const value = String(validated?.value ?? "").trim();
  return `${valueType}\u0000${value}`;
}

function normalizedDraftEditValue(draft) {
  return normalizedValidatedEditValue({
    valueType: draft?.valueType,
    value: draft?.summary,
  });
}

async function normalizedOriginalEditValue(node) {
  const value = editableValueForNode(node).trim();
  try {
    const validated = await invoke("validate_edit_value", { value });
    return normalizedValidatedEditValue(validated);
  } catch {
    const valueType = String(node?.valueType ?? node?.value_type ?? "").toLowerCase();
    return `${valueType}\u0000${String(node?.value ?? "").trim()}`;
  }
}

async function applyObjectValueEdit(options, inputValue) {
  const editPath = options?.editPath;
  const editReference = normalizeEditReference(options);
  const editKey = objectEditEntryKey(options);
  if (!editKey || !editReference) {
    return false;
  }
  if (!isEditableObjectValueType(options?.node)) {
    showInspectorEditError("Only Number, String, and Name values can be edited here.");
    return false;
  }

  const value = String(inputValue ?? "").trim();
  if (!value) {
    showInspectorEditError("Edited value cannot be empty.");
    return false;
  }

  try {
    const existingDraft = objectEditDrafts.get(editKey);
    const validated = await invoke("validate_edit_value", { value });
    if (!isEditableObjectValueType({ valueType: validated.valueType })) {
      showInspectorEditError("Edited value must remain a Number, String, or Name.");
      activeObjectEditKey = editKey;
      activeObjectEditPathKey = editKey;
      return false;
    }

    const normalizedValue = normalizedValidatedEditValue(validated);
    if (existingDraft && normalizedValue === normalizedDraftEditValue(existingDraft)) {
      activeObjectEditKey = null;
      activeObjectEditPathKey = null;
      renderObjectDetailsTree(activeObjectDetailsNode);
      return true;
    }

    const originalValue = await normalizedOriginalEditValue(options.node);
    if (normalizedValue === originalValue) {
      if (existingDraft) {
        objectEditDrafts.delete(editKey);
        markObjectDraftsChanged();
        updateObjectEditToolbar();
      }
      activeObjectEditKey = null;
      activeObjectEditPathKey = null;
      renderObjectDetailsTree(activeObjectDetailsNode);
      return true;
    }

    objectEditDrafts.set(editKey, {
      reference: editReference,
      path: editPath,
      input: value,
      valueType: validated.valueType,
      summary: validated.value,
    });
    markObjectDraftsChanged();
    activeObjectEditKey = null;
    activeObjectEditPathKey = null;
    updateObjectEditToolbar();
    renderObjectDetailsTree(activeObjectDetailsNode);
    return true;
  } catch (error) {
    showInspectorEditError(`Invalid PDF value: ${String(error)}`);
    activeObjectEditKey = editKey;
    activeObjectEditPathKey = editKey;
    return false;
  }
}

function showInspectorEditError(message) {
  elements.inspectorError.textContent = message;
  elements.inspectorError.hidden = false;
}

function updateObjectEditToolbar() {
  const total = objectEditDrafts.size + streamEditDrafts.size;
  const current = activeReference ? objectDraftEntries(referenceKey(activeReference)).length : 0;
  const dirty = total > 0;
  elements.objectEditStatus.textContent = dirty
    ? `${total} unsaved document edit${total === 1 ? "" : "s"} in this PDF. Ctrl+S saves the current file; Save As writes a separate copy.`
    : "Double-click visible Number, String, or Name values to edit. Other value types are read-only.";
  elements.objectEditStatus.classList.toggle("is-dirty", dirty);
  elements.revertObjectEdits.disabled = current === 0;
  elements.revertAllEdits.disabled = !dirty;
  elements.saveModifiedPdf.disabled = !dirty;
  if (elements.saveEditsAndRerender) {
    elements.saveEditsAndRerender.disabled = !dirty;
  }
  updateFileMenuState();
  updatePagePreviewDraftStatus();
}

function updatePagePreviewDraftStatus(message = null, options = {}) {
  if (!elements.pagePreviewDraftStatus) {
    return;
  }
  const dirty = hasDocumentDraftEdits();
  elements.pagePreviewDraftStatus.hidden = !dirty;
  if (!dirty) {
    elements.pagePreviewDraftTitle.textContent = "Draft preview";
    elements.pagePreviewDraftMessage.textContent = "";
    return;
  }

  const isSnapshot = options.snapshot === true;
  elements.pagePreviewDraftTitle.textContent = isSnapshot
    ? "Previewing unsaved draft"
    : "Unsaved object edits";
  elements.pagePreviewDraftMessage.textContent = message ?? (
    isSnapshot
      ? "This preview is rendered from a temporary local PDF snapshot. Save writes the current PDF; Save As writes a separate copy."
      : "Page Preview will render unsaved document edits through a temporary local PDF snapshot. Save writes the current PDF; Save As writes a separate copy."
  );
}

function objectDraftEntries(refKey) {
  return Array.from(objectEditDrafts.values()).filter((entry) => referenceKey(entry.reference) === refKey);
}

function revertCurrentObjectEdits() {
  if (!activeReference) {
    return;
  }
  const refKey = referenceKey(activeReference);
  for (const [key, draft] of objectEditDrafts.entries()) {
    if (referenceKey(draft.reference) === refKey) {
      objectEditDrafts.delete(key);
    }
  }
  streamEditDrafts.delete(refKey);
  if (activeStreamEditKey === refKey) {
    closeStreamEditPanel();
  }
  activeObjectEditKey = null;
  activeObjectEditPathKey = null;
  markObjectDraftsChanged();
  updateObjectEditToolbar();
  renderObjectDetailsTree(activeObjectDetailsNode);
  updateStreamEditControls();
}

function revertAllObjectEdits() {
  objectEditDrafts.clear();
  streamEditDrafts.clear();
  closeStreamEditPanel();
  activeObjectEditKey = null;
  activeObjectEditPathKey = null;
  markObjectDraftsChanged();
  updateObjectEditToolbar();
  renderObjectDetailsTree(activeObjectDetailsNode);
  updateStreamEditControls();
}

async function saveModifiedPdfAs(options = {}) {
  if (!currentPdfPath || !hasDocumentDraftEdits()) {
    showNoDraftSaveStatus();
    return;
  }

  const restorePageNumber = options.reRenderPageNumber ?? activePageNumber();
  let selected = options.outputPath;
  if (typeof selected !== "string") {
    selected = await save({
      defaultPath: defaultModifiedPdfPath(currentPdfPath),
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof selected !== "string") {
      return;
    }
  }

  elements.saveModifiedPdf.disabled = true;
  if (elements.saveEditsAndRerender) {
    elements.saveEditsAndRerender.disabled = true;
  }
  elements.objectEditStatus.textContent = "Saving modified PDF...";
  elements.objectEditStatus.classList.add("is-dirty");
  try {
    const edits = objectDraftsToTauriEdits();
    const streamEdits = streamDraftsToTauriEdits();
    const result = await invoke("save_modified_pdf_as", {
      path: currentPdfPath,
      outputPath: selected,
      edits,
      streamEdits,
    });
    resetObjectEditDrafts();
    resetObjectInspectionCache();
    clearPagePreview("Modified PDF saved. Reopening saved file...");
    await loadPdf(result.path ?? selected);
    if (options.reRender) {
      const selectedAfterReload = await selectPageNumberAfterLoad(restorePageNumber);
      if (!selectedAfterReload) {
        setActiveWorkspaceTab("page");
        clearPagePreviewImage("Modified PDF saved. Select a page to render the saved file.");
      }
    }
  } catch (error) {
    showInspectorEditError(`Save As failed: ${String(error)}`);
    updateObjectEditToolbar();
  }
}

async function saveModifiedPdfInPlace(options = {}) {
  setFileMenuOpen(false);
  if (!currentPdfPath || !hasDocumentDraftEdits()) {
    showNoDraftSaveStatus();
    return;
  }

  const restorePageNumber = options.reRenderPageNumber ?? activePageNumber();
  elements.saveModifiedPdf.disabled = true;
  if (elements.saveEditsAndRerender) {
    elements.saveEditsAndRerender.disabled = true;
  }
  elements.objectEditStatus.textContent = "Saving edits to the current PDF...";
  elements.objectEditStatus.classList.add("is-dirty");
  try {
    const edits = objectDraftsToTauriEdits();
    const streamEdits = streamDraftsToTauriEdits();
    const result = await invoke("save_modified_pdf_in_place", {
      path: currentPdfPath,
      edits,
      streamEdits,
    });
    const savedPath = result.path ?? currentPdfPath;
    resetObjectEditDrafts();
    resetObjectInspectionCache();
    clearPagePreview("Modified PDF saved in place. Reopening current file...");
    await loadPdf(savedPath);
    if (options.reRender) {
      const selectedAfterReload = await selectPageNumberAfterLoad(restorePageNumber);
      if (!selectedAfterReload) {
        setActiveWorkspaceTab("page");
        clearPagePreviewImage("Modified PDF saved. Select a page to render the saved file.");
      }
    }
  } catch (error) {
    showInspectorEditError(`Save failed: ${String(error)}`);
    updateObjectEditToolbar();
  }
}

function showNoDraftSaveStatus() {
  if (elements.objectEditStatus) {
    elements.objectEditStatus.textContent = "No unsaved edits to save.";
    elements.objectEditStatus.classList.remove("is-dirty");
  }
  showInfoNotice("No unsaved edits to save.");
}

async function saveModifiedPdfFromFileMenu() {
  setFileMenuOpen(false);
  await saveModifiedPdfInPlace();
}

async function saveModifiedPdfAsFromFileMenu() {
  setFileMenuOpen(false);
  await saveModifiedPdfAs();
}

function defaultModifiedPdfPath(path) {
  return String(path ?? "").replace(/\.pdf$/i, "-modified.pdf") || "modified.pdf";
}

function renderTrailerNodeRow(node, options) {
  return renderStructureNodeRow(node, options);
}

function renderStructureNodeRow(node, options) {
  const rowOptions = { ...options, node };
  const row = document.createElement("tr");
  row.className = "trailer-tree-row";
  const editKey = objectEditEntryKey(rowOptions);
  const draft = editKey ? objectEditDrafts.get(editKey) : null;
  if (rowOptions.isExpanded) {
    row.classList.add("is-expanded");
  }
  if (rowOptions.isLoading) {
    row.classList.add("is-loading");
    row.setAttribute("aria-busy", "true");
  }
  if (draft) {
    row.classList.add("is-edited");
  }

  const keyCell = document.createElement("td");
  keyCell.className = "trailer-key";
  const keyWrap = document.createElement("div");
  keyWrap.className = "trailer-key-wrap";
  keyWrap.style.setProperty("--depth", String(rowOptions.depth));

  if (rowOptions.canExpand) {
    const toggle = document.createElement("button");
    toggle.className = "trailer-toggle";
    if (rowOptions.isLoading) {
      toggle.classList.add("is-loading");
    }
    toggle.type = "button";
    toggle.textContent = rowOptions.isExpanded ? "▾" : "▸";
    toggle.title = rowOptions.isLoading
      ? `Loading ${objectLabel(rowOptions.reference)}`
      : rowOptions.isExpanded
        ? "Collapse"
        : "Expand";
    toggle.setAttribute("aria-label", toggle.title);
    toggle.addEventListener("click", () => toggleStructureNode(rowOptions.rowKey, rowOptions.tree ?? trailerTreeContext()));
    keyWrap.appendChild(toggle);
  } else {
    const spacer = document.createElement("span");
    spacer.className = "trailer-toggle-spacer";
    spacer.textContent = "•";
    keyWrap.appendChild(spacer);
  }

  const keyText = document.createElement("span");
  keyText.className = "trailer-key-text";
  keyText.textContent = node.key || "-";
  keyText.title = node.key || "-";
  keyWrap.appendChild(keyText);
  keyCell.appendChild(keyWrap);
  row.appendChild(keyCell);

  row.appendChild(tableCell(formatTrailerType(node.valueType ?? node.value_type), "trailer-type"));

  const value = document.createElement("td");
  value.className = "trailer-value";
  if (canEditObjectDetailsValue(rowOptions) && editKey) {
    value.classList.add("is-editable");
    value.title = "Double-click to edit this value.";
    value.addEventListener("dblclick", () => beginObjectValueEdit(rowOptions));
  }
  if (activeObjectEditKey === editKey && editKey) {
    value.appendChild(renderObjectEditForm(node, rowOptions, draft));
  } else if (rowOptions.reference) {
    const wrap = document.createElement("div");
    wrap.className = "trailer-value-wrap";
    const button = document.createElement("button");
    button.className = "trailer-reference-button";
    button.type = "button";
    button.textContent = draft?.summary || node.value || objectLabel(rowOptions.reference);
    button.title = `Inspect ${draft?.summary || node.value || objectLabel(rowOptions.reference)}`;
    button.addEventListener("click", () => openObjectWorkspaceTab(rowOptions.reference));
    button.addEventListener("dblclick", (event) => event.stopPropagation());
    wrap.appendChild(button);
    if (rowOptions.isLoading) {
      const loading = document.createElement("span");
      loading.className = "trailer-loading-pill";
      loading.textContent = "loading";
      wrap.appendChild(loading);
    }
    value.appendChild(wrap);
  } else {
    value.appendChild(renderObjectValueDisplay(node, rowOptions, draft));
  }
  row.appendChild(value);
  return row;
}

function renderObjectValueDisplay(node, options, draft) {
  const wrap = document.createElement("div");
  wrap.className = "trailer-value-wrap";

  const text = document.createElement("span");
  text.className = "trailer-value-text";
  text.textContent = draft?.summary || node.value || "-";
  text.title = draft?.input || draft?.summary || node.value || "-";
  wrap.appendChild(text);

  if (canEditObjectDetailsValue(options)) {
    text.classList.add("is-editable");
  }

  return wrap;
}

function beginObjectValueEdit(options) {
  const editKey = objectEditEntryKey(options);
  if (!editKey || !canEditObjectDetailsValue(options)) {
    return;
  }
  activeObjectEditKey = editKey;
  activeObjectEditPathKey = editKey;
  renderObjectDetailsTree(activeObjectDetailsNode);
}

function renderObjectEditForm(node, options, draft) {
  const form = document.createElement("form");
  form.className = "object-edit-form";
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    commitObjectEditInput(options, input);
  });

  const input = document.createElement("input");
  input.className = "object-edit-input";
  input.type = "text";
  input.value = draft?.input ?? editableValueForNode(node);
  input.title = "Enter a valid PDF Number, String, or Name, for example 65, /FlateDecode, or (Text).";
  form.appendChild(input);
  input.addEventListener("blur", () => commitObjectEditInput(options, input));
  input.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      activeObjectEditKey = null;
      activeObjectEditPathKey = null;
      renderObjectDetailsTree(activeObjectDetailsNode);
    }
  });

  window.setTimeout(() => {
    input.focus();
    input.select();
  }, 0);
  return form;
}

function commitObjectEditInput(options, input) {
  if (input.dataset.committing === "true") {
    return;
  }
  input.dataset.committing = "true";
  applyObjectValueEdit(options, input.value).then((ok) => {
    if (!ok && document.body.contains(input)) {
      window.setTimeout(() => input.focus(), 0);
    }
  }).finally(() => {
    input.dataset.committing = "false";
  });
}

function appendTrailerMessageRow(message, depth, kind, state) {
  if (state.rows >= TRAILER_TREE_MAX_ROWS) {
    state.limitHit = true;
    return;
  }
  const row = document.createElement("tr");
  row.className = "trailer-message-row";
  row.dataset.kind = kind;
  const cell = document.createElement("td");
  cell.colSpan = 3;
  const wrap = document.createElement("div");
  wrap.className = "trailer-key-wrap trailer-message";
  wrap.style.setProperty("--depth", String(depth));
  wrap.textContent = message;
  cell.appendChild(wrap);
  row.appendChild(cell);
  state.rowsList.push({ kind: "element", element: row });
  state.rows += 1;
}

function appendTrailerShowMoreRow({ rowKey, batchKey, depth, shown, total, state }) {
  if (state.rows >= TRAILER_TREE_MAX_ROWS) {
    state.limitHit = true;
    return;
  }
  const row = document.createElement("tr");
  row.className = "trailer-message-row";
  row.dataset.kind = "more";
  const cell = document.createElement("td");
  cell.colSpan = 3;
  const wrap = document.createElement("div");
  wrap.className = "trailer-key-wrap trailer-message";
  wrap.style.setProperty("--depth", String(depth));
  const button = document.createElement("button");
  button.className = "tree-show-more";
  button.type = "button";
  const tree = state.tree ?? trailerTreeContext();
  button.textContent = `Show ${Math.min(tree.batchSize, total - shown)} more (${shown}/${total})`;
  button.addEventListener("click", () => {
    tree.visibleChildCounts.set(batchKey, Math.min(total, shown + tree.batchSize));
    tree.render();
  });
  wrap.appendChild(button);
  cell.appendChild(wrap);
  row.appendChild(cell);
  state.rowsList.push({ kind: "element", element: row });
  state.rows += 1;
}

function normalizeTrailerNode(node) {
  return {
    kind: node.kind ?? "value",
    key: node.key ?? "-",
    valueType: node.valueType ?? node.value_type ?? "-",
    value: node.value ?? "-",
    reference: node.reference ?? null,
    expandable: Boolean(node.expandable),
    children: Array.isArray(node.children) ? node.children : [],
    stream: Boolean(node.stream),
  };
}

function trailerNodeKey(node, fallback) {
  const normalized = normalizeTrailerNode(node);
  const ref = normalized.reference ? `:${referenceKey(normalized.reference)}` : "";
  return `${normalized.kind}:${normalized.key}:${normalized.valueType}:${normalized.value}${ref}` || fallback;
}

function toggleTrailerNode(rowKey) {
  toggleStructureNode(rowKey, trailerTreeContext());
}

function toggleStructureNode(rowKey, tree) {
  if (tree.expandedKeys.has(rowKey)) {
    tree.expandedKeys.delete(rowKey);
  } else {
    tree.expandedKeys.add(rowKey);
  }
  tree.render();
}

async function loadTrailerReference(reference) {
  return loadStructureReference(reference, trailerTreeContext());
}

async function loadStructureReference(reference, tree = trailerTreeContext()) {
  if (!currentPdfPath || !reference) {
    return;
  }
  const requestPath = currentPdfPath;
  const requestGeneration = pdfOpenGeneration;
  const normalized = normalizeReference(reference);
  const key = referenceKey(normalized);
  if (tree.loadedObjects.has(key) || tree.loadingObjects.has(key)) {
    return;
  }

  tree.loadingObjects.add(key);
  tree.loadErrors.delete(key);
  const perf = perfMark(tree.loadPerfName, objectLabel(normalized));
  try {
    const result = await scheduleTauriTask(
      tree.loadTaskKind,
      "load_trailer_object",
      {
        path: requestPath,
        object: normalized.object,
        generation: normalized.generation,
      },
      {
        dropGroup: tree.loadTaskDropGroup,
        coalesceKey: `${tree.id}:object:${requestPath}:${key}`,
      },
    );
    if (requestGeneration !== pdfOpenGeneration || requestPath !== currentPdfPath) {
      perfDone(perf, "stale");
      return;
    }
    perfDone(perf);
    tree.loadedObjects.set(key, result.node);
  } catch (error) {
    if (requestGeneration !== pdfOpenGeneration || requestPath !== currentPdfPath) {
      perfDone(perf, "stale-error");
      return;
    }
    perfDone(perf, "error");
    tree.loadErrors.set(key, `Could not load ${objectLabel(normalized)}: ${String(error)}`);
  } finally {
    if (requestGeneration === pdfOpenGeneration && requestPath === currentPdfPath) {
      tree.loadingObjects.delete(key);
      tree.render();
    }
  }
}

function formatTrailerType(type) {
  if (!type) {
    return "-";
  }
  return String(type)
    .split(/[\s_-]+/)
    .filter(Boolean)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}

function renderObjectTree(tree) {
  elements.objectTree.replaceChildren();
  treeButtonsByReference = new Map();
  objectTreeVisibleChildCounts = new Map();
  objectTreeExpandedKeys = new Set();
  activeObjectTree = tree ?? null;
  elements.objectTree.onscroll = null;
  virtualObjectTreeState = null;
  if (!tree) {
    clearObjectTreeSearch();
    elements.objectTree.textContent = "No object tree available.";
    elements.objectTree.classList.add("empty-tree");
    elements.treeCount.textContent = "0";
    return;
  }

  elements.objectTree.classList.remove("empty-tree");
  elements.treeCount.textContent = countObjectNodes(tree).toString();
  seedObjectTreeExpandedKeys(tree);
  refreshObjectTreeSearchRows();
  renderVirtualObjectTree();
  elements.objectTree.onscroll = () => scheduleVirtualRender(virtualObjectTreeState);
}

function seedObjectTreeExpandedKeys(tree) {
  objectTreeExpandedKeys.add("root");
  const children = Array.isArray(tree?.children) ? tree.children : [];
  children.forEach((child, index) => {
    if ((child.children ?? []).length > 0) {
      objectTreeExpandedKeys.add(`root>${index}:${treeNodeKey(child)}`);
    }
  });
}

function renderVirtualObjectTree() {
  if (!activeObjectTree) {
    return;
  }
  treeButtonsByReference = new Map();
  const isSearching = objectTreeSearchQuery.trim().length > 0;
  if (isSearching) {
    refreshObjectTreeSearchRows();
  }
  const rows = isSearching ? objectTreeSearchRows : flattenObjectTreeRows(activeObjectTree);
  updateObjectTreeSearchStatus();
  const range = virtualRange(elements.objectTree, rows.length, OBJECT_TREE_ROW_HEIGHT);
  const fragment = document.createDocumentFragment();
  if (rows.length === 0 && isSearching) {
    fragment.appendChild(renderObjectTreeEmptySearchRow());
  }
  if (range.enabled && range.before) {
    fragment.appendChild(virtualSpacer(range.before));
  }
  for (let index = range.start; index < range.end; index += 1) {
    fragment.appendChild(renderObjectTreeRow(rows[index]));
  }
  if (range.enabled && range.after) {
    fragment.appendChild(virtualSpacer(range.after));
  }
  elements.objectTree.replaceChildren(fragment);
  if (activeReference) {
    selectedTreeButton = treeButtonsByReference.get(referenceKey(activeReference)) ?? null;
    selectedTreeButton?.classList.add("is-selected");
  }
  virtualObjectTreeState = {
    frame: null,
    render: renderVirtualObjectTree,
  };
}

function objectTreeSearchTokens() {
  return objectTreeSearchQuery
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean);
}

function refreshObjectTreeSearchRows() {
  if (!activeObjectTree) {
    objectTreeSearchRows = [];
    return;
  }
  const tokens = objectTreeSearchTokens();
  if (!tokens.length) {
    objectTreeSearchRows = [];
    return;
  }
  const rows = [];
  const visit = (node, depth, path, ancestors) => {
    const haystack = objectTreeSearchText(node, ancestors);
    if (tokens.every((token) => haystack.includes(token))) {
      rows.push({
        kind: "search",
        node,
        depth: Math.min(depth, 4),
        path,
        hasChildren: Array.isArray(node.children) && node.children.length > 0,
        expanded: false,
        context: ancestors.slice(-2).join(" / "),
      });
    }
    const nextAncestors = [...ancestors, node.label ?? objectLabel(node.object)];
    const children = Array.isArray(node.children) ? node.children : [];
    for (let index = 0; index < children.length; index += 1) {
      visit(children[index], depth + 1, `${path}>${index}:${treeNodeKey(children[index])}`, nextAncestors);
    }
  };
  visit(activeObjectTree, 0, "root", []);
  objectTreeSearchRows = rows;
}

function objectTreeSearchText(node, ancestors) {
  const reference = node.object ? normalizeReference(node.object) : null;
  return [
    node.label,
    node.kind,
    node.summary,
    reference ? objectLabel(reference) : "",
    reference ? `${reference.object}` : "",
    reference ? `${reference.object} ${reference.generation}` : "",
    ...ancestors,
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
}

function updateObjectTreeSearchStatus() {
  if (!elements.objectTreeSearchStatus) {
    return;
  }
  const query = objectTreeSearchQuery.trim();
  if (!query || !activeObjectTree) {
    elements.objectTreeSearchStatus.hidden = true;
    elements.objectTreeSearchStatus.textContent = "";
    return;
  }
  const count = objectTreeSearchRows.length;
  elements.objectTreeSearchStatus.hidden = false;
  elements.objectTreeSearchStatus.textContent =
    count === 0 ? "No matching objects." : `${count} matching object${count === 1 ? "" : "s"}.`;
}

function renderObjectTreeEmptySearchRow() {
  const row = document.createElement("div");
  row.className = "tree-row tree-empty-search-row";
  row.style.setProperty("--depth", "0");
  row.textContent = "No matching objects.";
  return row;
}

function flattenObjectTreeRows(root) {
  const rows = [];
  let limitHit = false;
  const visit = (node, depth, path) => {
    if (limitHit) {
      return;
    }
    if (rows.length >= OBJECT_TREE_RENDER_BUDGET) {
      rows.push({ kind: "limit", depth, path: `${path}>limit` });
      limitHit = true;
      return;
    }
    const children = Array.isArray(node.children) ? node.children : [];
    const hasChildren = children.length > 0;
    const expanded = objectTreeExpandedKeys.has(path);
    rows.push({ kind: "node", node, depth, path, hasChildren, expanded });
    if (!hasChildren || !expanded) {
      return;
    }
    const batchKey = childBatchKey("object-tree", path);
    const visibleChildren = Math.min(children.length, getChildBatchSize(objectTreeVisibleChildCounts, batchKey, OBJECT_TREE_CHILD_BATCH));
    for (let index = 0; index < visibleChildren; index += 1) {
      visit(children[index], depth + 1, `${path}>${index}:${treeNodeKey(children[index])}`);
      if (limitHit) {
        return;
      }
    }
    if (visibleChildren < children.length) {
      rows.push({
        kind: "more",
        depth: depth + 1,
        path: `${path}>more`,
        batchKey,
        shown: visibleChildren,
        total: children.length,
      });
    }
  };
  visit(root, 0, "root");
  return rows;
}

function renderObjectTreeRow(rowInfo) {
  if (rowInfo.kind === "more") {
    return renderObjectTreeShowMoreRow(rowInfo);
  }
  if (rowInfo.kind === "limit") {
    return renderObjectTreeLimitRow(rowInfo.depth);
  }
  const { node, depth, path, hasChildren, expanded } = rowInfo;
  const row = document.createElement(node.object ? "button" : "div");
  row.className = "tree-row";
  if (rowInfo.kind === "search") {
    row.classList.add("is-search-result");
  }
  row.style.setProperty("--depth", String(depth));
  row.dataset.kind = node.kind;

  const marker = document.createElement("span");
  marker.className = "tree-marker";
  marker.textContent = rowInfo.kind === "search" ? "•" : hasChildren ? (expanded ? "▾" : "▸") : "•";
  row.appendChild(marker);

  const label = document.createElement("span");
  label.className = "tree-label";
  label.textContent = node.label;
  if (rowInfo.context) {
    label.title = `${rowInfo.context} / ${node.label}`;
  }
  row.appendChild(label);

  if (node.object) {
    const ref = document.createElement("span");
    ref.className = "tree-ref";
    ref.textContent = objectLabel(node.object);
    row.appendChild(ref);
    row.type = "button";
    if (!treeButtonsByReference.has(referenceKey(node.object))) {
      treeButtonsByReference.set(referenceKey(node.object), row);
    }
    row.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      openObjectWorkspaceTab(node.object);
    });
  }

  if (node.summary) {
    row.title = node.summary;
  }

  if (hasChildren && rowInfo.kind !== "search") {
    row.addEventListener("click", (event) => {
      if (node.object) {
        return;
      }
      event.preventDefault();
      toggleObjectTreeNode(path);
    });
    marker.addEventListener("click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      toggleObjectTreeNode(path);
    });
  }
  return row;
}

function toggleObjectTreeNode(path) {
  if (objectTreeExpandedKeys.has(path)) {
    objectTreeExpandedKeys.delete(path);
  } else {
    objectTreeExpandedKeys.add(path);
  }
  renderVirtualObjectTree();
}

function renderObjectTreeShowMoreRow({ depth, batchKey, shown, total }) {
  const button = document.createElement("button");
  button.className = "tree-row tree-show-more-row";
  button.type = "button";
  button.style.setProperty("--depth", String(depth));
  button.textContent = `Show ${Math.min(OBJECT_TREE_CHILD_BATCH, total - shown)} more (${shown}/${total})`;
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    objectTreeVisibleChildCounts.set(batchKey, Math.min(total, shown + OBJECT_TREE_CHILD_BATCH));
    renderVirtualObjectTree();
  });
  return button;
}

function renderObjectTreeLimitRow(depth) {
  const row = document.createElement("div");
  row.className = "tree-row tree-limit-row";
  row.style.setProperty("--depth", String(depth));
  row.textContent = "Tree render budget reached. Collapse branches or inspect objects directly.";
  return row;
}

function treeNodeKey(node) {
  const reference = node.object ? referenceKey(node.object) : "";
  return `${node.kind ?? "node"}:${node.label ?? ""}:${reference}`;
}

function findObjectTreePathForReference(node, reference, path = "root") {
  if (!node || !reference) {
    return null;
  }
  if (node.object && sameReference(node.object, reference)) {
    return path;
  }
  const children = Array.isArray(node.children) ? node.children : [];
  for (let index = 0; index < children.length; index += 1) {
    const childPath = `${path}>${index}:${treeNodeKey(children[index])}`;
    const found = findObjectTreePathForReference(children[index], reference, childPath);
    if (found) {
      return found;
    }
  }
  return null;
}

function expandObjectTreePath(path) {
  const parts = path.split(">");
  let current = "";
  for (const part of parts) {
    current = current ? `${current}>${part}` : part;
    objectTreeExpandedKeys.add(current);
  }
}

function scrollObjectTreePathIntoView(path) {
  const rows = flattenObjectTreeRows(activeObjectTree);
  const index = rows.findIndex((row) => row.path === path);
  if (index < 0) {
    renderVirtualObjectTree();
    return;
  }
  const top = index * OBJECT_TREE_ROW_HEIGHT;
  const bottom = top + OBJECT_TREE_ROW_HEIGHT;
  const viewportBottom = elements.objectTree.scrollTop + elements.objectTree.clientHeight;
  if (top < elements.objectTree.scrollTop || bottom > viewportBottom) {
    elements.objectTree.scrollTop = Math.max(0, top - OBJECT_TREE_ROW_HEIGHT);
  }
  renderVirtualObjectTree();
}

function scrollObjectTreeIndexIntoView(index) {
  if (index < 0) {
    renderVirtualObjectTree();
    return;
  }
  const top = index * OBJECT_TREE_ROW_HEIGHT;
  const bottom = top + OBJECT_TREE_ROW_HEIGHT;
  const viewportBottom = elements.objectTree.scrollTop + elements.objectTree.clientHeight;
  if (top < elements.objectTree.scrollTop || bottom > viewportBottom) {
    elements.objectTree.scrollTop = Math.max(0, top - OBJECT_TREE_ROW_HEIGHT);
  }
  renderVirtualObjectTree();
}

async function navigateToObject(reference, options = {}) {
  if (!currentPdfPath) {
    return;
  }

  const normalized = options.pushHistory === false ? normalizeReference(reference) : pushNavigation(reference);
  if (options.activateTab !== false) {
    openObjectWorkspaceTab(normalized, { pushHistory: false });
  }
  activeReference = normalized;
  setSelectedTreeReference(normalized);
  elements.inspectorError.hidden = true;
  elements.inspectorError.textContent = "";
  inspectedStreamReference = null;

  const requestId = ++navigationRequestId;
  const cacheKey = referenceKey(normalized);
  const cachedInspection = objectInspectionCache.get(cacheKey);
  if (cachedInspection) {
    elements.inspectorState.textContent = "Ready";
    elements.inspectorState.classList.remove("is-loading");
    renderInspection(cachedInspection);
    return;
  }

  elements.inspectorState.textContent = "Loading";
  elements.inspectorState.classList.add("is-loading");
  const perf = perfMark("inspect_object", objectLabel(normalized));
  try {
    const inspection = await loadObjectInspection(normalized);
    if (requestId !== navigationRequestId) {
      perfDone(perf, "stale");
      return;
    }
    perfDone(perf);
    renderInspection(inspection);
  } catch (error) {
    if (requestId !== navigationRequestId) {
      perfDone(perf, "stale-error");
      return;
    }
    perfDone(perf, "error");
    elements.inspectorState.textContent = "Error";
    elements.inspectorSubtitle.textContent = objectLabel(activeReference);
    elements.inspectorError.textContent = String(error);
    elements.inspectorError.hidden = false;
  } finally {
    if (requestId === navigationRequestId) {
      elements.inspectorState.classList.remove("is-loading");
    }
  }
}

async function loadObjectInspection(reference) {
  const normalized = normalizeReference(reference);
  const cacheKey = referenceKey(normalized);
  const cachedInspection = objectInspectionCache.get(cacheKey);
  if (cachedInspection) {
    return cachedInspection;
  }
  const pendingInspection = objectInspectionLoading.get(cacheKey);
  if (pendingInspection) {
    return pendingInspection;
  }

  const requestPath = currentPdfPath;
  let request = null;
  request = scheduleTauriTask(
    "objectInspect",
    "inspect_object",
    {
      path: requestPath,
      object: normalized.object,
      generation: normalized.generation,
    },
    {
      dropGroup: "objectInspect",
      coalesceKey: `objectInspect:${requestPath}:${cacheKey}`,
    },
  )
    .then((inspection) => {
      if (requestPath === currentPdfPath) {
        objectInspectionCache.set(cacheKey, inspection);
      }
      return inspection;
    })
    .finally(() => {
      if (objectInspectionLoading.get(cacheKey) === request) {
        objectInspectionLoading.delete(cacheKey);
      }
    });
  objectInspectionLoading.set(cacheKey, request);
  return request;
}

function renderInspection(inspection) {
  activeObjectInspection = inspection;
  const reference = inspection.reference;
  elements.inspectorState.textContent = "Ready";
  elements.inspectorSubtitle.textContent = objectLabel(reference);
  elements.inspectorReference.textContent = objectLabel(reference);
  elements.inspectorType.textContent = inspection.object_type ?? "-";
  elements.inspectorSummary.textContent = inspection.value_summary ?? "-";
  elements.inspectorRange.textContent = inspection.raw_range
    ? `${inspection.raw_range.start}..${inspection.raw_range.end}`
    : "-";
  elements.inspectorRawLength.textContent = inspection.raw_length ?? "-";
  elements.inspectorKeys.textContent =
    inspection.dictionary_keys?.length ? inspection.dictionary_keys.join(", ") : "-";
  resetObjectDetailsTreeState();
  renderObjectDetailsTree(inspection.objectNode ?? null, { expandRoot: true });
  updateObjectEditToolbar();
  renderStreamInspection(inspection.stream, reference);
  if (inspection.stream && !sameReference(activeStreamReference, reference)) {
    clearStreamViewer("Open Stream Details to inspect hex or decoded bytes.");
  } else if (!inspection.stream) {
    clearStreamViewer("Selected object has no stream.");
  }
}

function renderStreamInspection(stream, reference) {
  elements.streamSection.hidden = !stream;
  if (!stream) {
    elements.openStreamDetails.disabled = true;
    elements.openStreamDetails.title = "Select a stream object to inspect stream details.";
    inspectedStreamReference = null;
    return;
  }

  inspectedStreamReference = normalizeReference(reference);
  elements.openStreamDetails.disabled = false;
  elements.openStreamDetails.title = `Open stream details for ${objectLabel(inspectedStreamReference)}`;
  elements.streamDeclaredLength.textContent = stream.declared_length ?? "Unknown";
  elements.streamActualLength.textContent = stream.actual_length ?? "-";
  elements.streamFilters.textContent = stream.filters?.length ? stream.filters.join(" -> ") : "None";
  elements.streamDecodedLength.textContent = stream.decoded_length ?? "Unavailable";
  elements.streamDecodeIssues.textContent = stream.decode_issues?.length
    ? stream.decode_issues.map((issue) => `${issue.filter}: ${issue.message}`).join("; ")
    : "None";
}

async function loadStreamView(reference) {
  if (!currentPdfPath) {
    return;
  }

  const requestId = ++streamViewRequestId;
  const normalized = normalizeReference(reference);
  const requestPath = currentPdfPath;
  const cacheKey = streamCacheKey(currentPdfPath, normalized);
  const cachedView = streamViewCache.get(cacheKey);
  activeStreamView = null;
  clearContentAnalysis();
  clearStreamImagePreview();
  activeStreamReference = normalized;
  elements.streamViewerSubtitle.textContent = objectLabel(normalized);
  if (cachedView) {
    activeStreamView = cachedView;
    activeStreamReference = normalizeReference(cachedView.reference);
    applyStreamDraftToView(activeStreamView);
    renderStreamViewMetadata(cachedView);
    updateStreamActionButtons();
    updateStreamEditControls();
    updateStreamImageControls();
    renderStreamViewerContent();
    loadActiveStreamPreview();
    if (activeStreamMode === "decoded" && shouldRunContentAnalysis(cachedView)) {
      loadContentAnalysis(normalized);
    }
    return;
  }

  elements.streamViewerState.textContent = "Loading metadata";
  elements.streamViewerState.classList.add("is-loading");
  elements.streamViewerError.hidden = true;
  elements.streamViewerError.textContent = "";
  clearStreamViewerStatus();
  setStreamViewerPlainText("Loading stream metadata...");
  updateStreamActionButtons();
  updateStreamImageControls();

  const metadataPerf = perfMark("view_stream_metadata", objectLabel(normalized));
  try {
    const metadata = await loadStreamMetadataFromCacheOrTauri(requestPath, normalized);
    if (requestId !== streamViewRequestId) {
      perfDone(metadataPerf, "stale");
      return;
    }
    perfDone(metadataPerf);
    activeStreamView = streamMetadataToPendingView(metadata);
    activeStreamReference = normalizeReference(metadata.reference);
    applyStreamDraftToView(activeStreamView);
    renderStreamViewMetadata(activeStreamView, { pendingPreview: true });
    updateStreamActionButtons();
    updateStreamEditControls();
    updateStreamImageControls();
    renderStreamViewerContent();
    loadActiveStreamPreview();
  } catch (error) {
    if (requestId !== streamViewRequestId) {
      perfDone(metadataPerf, "stale-error");
      return;
    }
    perfDone(metadataPerf, "error");
    activeStreamView = null;
    activeStreamReference = null;
    elements.streamViewerState.textContent = "Error";
    elements.streamViewerError.textContent = String(error);
    elements.streamViewerError.hidden = false;
    setStreamViewerPlainText("Stream metadata could not be loaded.");
    updateStreamActionButtons();
    updateStreamImageControls();
    clearContentAnalysis();
    elements.streamViewerState.classList.remove("is-loading");
    return;
  }
}

async function loadStreamMetadataFromCacheOrTauri(path, reference) {
  const normalized = normalizeReference(reference);
  const cacheKey = streamCacheKey(path, normalized);
  const cachedMetadata = streamMetadataCache.get(cacheKey);
  if (cachedMetadata) {
    return cachedMetadata;
  }
  const pendingMetadata = streamMetadataLoading.get(cacheKey);
  if (pendingMetadata) {
    return pendingMetadata;
  }
  let request = null;
  request = scheduleTauriTask(
    "streamMetadata",
    "view_stream_metadata",
    {
      path,
      object: normalized.object,
      generation: normalized.generation,
    },
    {
      dropGroup: "streamMetadata",
      coalesceKey: `streamMetadata:${path}:${cacheKey}`,
    },
  )
    .then((metadata) => {
      if (path === currentPdfPath) {
        streamMetadataCache.set(cacheKey, metadata);
      }
      return metadata;
    })
    .finally(() => {
      if (streamMetadataLoading.get(cacheKey) === request) {
        streamMetadataLoading.delete(cacheKey);
      }
    });
  streamMetadataLoading.set(cacheKey, request);
  return request;
}

async function loadStreamViewFromCacheOrTauri(path, reference) {
  const normalized = normalizeReference(reference);
  const cacheKey = streamCacheKey(path, normalized);
  const cachedView = streamViewCache.get(cacheKey);
  if (cachedView) {
    return cachedView;
  }
  const pendingView = streamViewLoading.get(cacheKey);
  if (pendingView) {
    return pendingView;
  }
  let request = null;
  request = scheduleTauriTask(
    "streamPreview",
    "view_stream",
    {
      path,
      object: normalized.object,
      generation: normalized.generation,
    },
    {
      dropGroup: "streamPreview",
      coalesceKey: `streamView:${path}:${cacheKey}`,
    },
  )
    .then((view) => {
      if (path === currentPdfPath) {
        streamViewCache.set(cacheKey, view);
      }
      return view;
    })
    .finally(() => {
      if (streamViewLoading.get(cacheKey) === request) {
        streamViewLoading.delete(cacheKey);
      }
    });
  streamViewLoading.set(cacheKey, request);
  return request;
}

async function loadActiveStreamPreview() {
  if (!currentPdfPath || !activeStreamReference || !activeStreamView) {
    return;
  }
  const mode = activeStreamMode;
  if (streamPreviewTextForMode(activeStreamView, mode) != null || streamPreviewErrorForMode(activeStreamView, mode)) {
    renderStreamViewerContent();
    updateStreamActionButtons();
    updateStreamImageControls();
    return;
  }

  const requestId = streamViewRequestId;
  const requestPath = currentPdfPath;
  const reference = normalizeReference(activeStreamReference);
  elements.streamViewerState.textContent = `Loading ${mode}`;
  elements.streamViewerState.classList.add("is-loading");
  renderStreamViewerContent();

  const perf = perfMark("view_stream_preview", `${objectLabel(reference)} mode=${mode}`);
  try {
    const preview = await loadStreamPreviewFromCacheOrTauri(requestPath, reference, mode);
    if (
      requestId !== streamViewRequestId ||
      requestPath !== currentPdfPath ||
      referenceKey(reference) !== referenceKey(activeStreamReference)
    ) {
      perfDone(perf, "stale");
      return;
    }
    perfDone(perf);
    mergeStreamPreview(activeStreamView, preview);
    applyStreamDraftToView(activeStreamView);
    renderStreamViewMetadata(activeStreamView);
    renderStreamViewerContent();
    updateStreamEditControls();
    updateStreamActionButtons();
    updateStreamImageControls();
    if (mode === "decoded" && shouldRunContentAnalysis(activeStreamView)) {
      loadContentAnalysis(reference);
    } else if (mode === "decoded") {
      clearContentAnalysis();
    }
  } catch (error) {
    if (
      requestId !== streamViewRequestId ||
      requestPath !== currentPdfPath ||
      referenceKey(reference) !== referenceKey(activeStreamReference)
    ) {
      perfDone(perf, "stale-error");
      return;
    }
    perfDone(perf, "error");
    mergeStreamPreview(activeStreamView, {
      reference,
      mode,
      text: null,
      error: String(error),
      warnings: [],
      decodeIssues: [],
      truncated: false,
    });
    renderStreamViewMetadata(activeStreamView);
    renderStreamViewerContent();
    updateStreamEditControls();
    updateStreamActionButtons();
    updateStreamImageControls();
    if (mode === "decoded") {
      clearContentAnalysis();
    }
  } finally {
    if (requestId === streamViewRequestId) {
      elements.streamViewerState.classList.remove("is-loading");
      elements.streamViewerState.textContent = "Metadata ready";
    }
  }
}

async function loadStreamPreviewFromCacheOrTauri(path, reference, mode) {
  const normalized = normalizeReference(reference);
  const cacheKey = streamPreviewCacheKey(path, normalized, mode);
  const cachedPreview = streamPreviewCache.get(cacheKey);
  if (cachedPreview) {
    return cachedPreview;
  }
  const pendingPreview = streamPreviewLoading.get(cacheKey);
  if (pendingPreview) {
    return pendingPreview;
  }
  let request = null;
  request = scheduleTauriTask(
    "streamPreview",
    "view_stream_preview",
    {
      path,
      object: normalized.object,
      generation: normalized.generation,
      mode,
    },
    {
      dropGroup: "streamPreview",
      coalesceKey: `streamPreview:${path}:${referenceKey(normalized)}:${mode}`,
    },
  )
    .then((preview) => {
      if (path === currentPdfPath) {
        streamPreviewCache.set(cacheKey, preview);
      }
      return preview;
    })
    .finally(() => {
      if (streamPreviewLoading.get(cacheKey) === request) {
        streamPreviewLoading.delete(cacheKey);
      }
    });
  streamPreviewLoading.set(cacheKey, request);
  return request;
}

function streamMetadataToPendingView(metadata) {
  return {
    reference: metadata.reference,
    declaredLength: metadata.declaredLength,
    rawLength: metadata.rawLength,
    rawRange: metadata.rawRange,
    decodedLength: null,
    filters: metadata.filters ?? [],
    decodeIssues: metadata.decodeIssues ?? [],
    warnings: metadata.warnings ?? [],
    image: metadata.image ?? null,
    rawTextTruncated: false,
    hexTextTruncated: false,
    decodedTextTruncated: false,
    previewLimit: null,
    canExportRaw: metadata.canExportRaw !== false,
    canExportDecoded: false,
    rawText: null,
    hexText: null,
    decodedText: null,
    rawError: null,
    hexError: null,
    decodedError: STREAM_DECODED_PENDING_MESSAGE,
    rawTextTruncatedForUi: false,
    hexTextTruncatedForUi: false,
    decodedTextTruncatedForUi: false,
    rawTextBinaryForUi: false,
    decodedTextBinaryForUi: false,
    metadataOnly: true,
  };
}

function mergeStreamPreview(view, preview) {
  if (!view || !preview) {
    return;
  }
  const mode = preview.mode;
  view.previewLimit = preview.previewLimit ?? view.previewLimit;
  view.canExportRaw = preview.canExportRaw ?? view.canExportRaw;
  view.canExportDecoded = preview.canExportDecoded ?? view.canExportDecoded;
  view.warnings = mergeUniqueStrings(view.warnings, preview.warnings);
  view.decodeIssues = preview.decodeIssues?.length ? preview.decodeIssues : view.decodeIssues;
  if (preview.decodedLength != null) {
    view.decodedLength = preview.decodedLength;
  }
  if (mode === "raw") {
    view.rawText = preview.text;
    view.rawTextTruncated = Boolean(preview.truncated);
    view.rawTextBinaryForUi = rawTextLooksBinary(preview.text);
    view.rawTextTruncatedForUi =
      streamTextTruncatedForUi(preview.text) || view.rawTextBinaryForUi;
    view.rawError = preview.error ?? null;
  } else if (mode === "hex") {
    view.hexText = preview.text;
    view.hexTextTruncated = Boolean(preview.truncated);
    view.hexTextTruncatedForUi = streamTextTruncatedForUi(preview.text);
    view.hexError = preview.error ?? null;
  } else if (mode === "decoded") {
    view.decodedText = preview.text;
    view.decodedTextTruncated = Boolean(preview.truncated);
    view.decodedTextBinaryForUi = rawTextLooksBinary(preview.text);
    view.decodedTextTruncatedForUi =
      streamTextTruncatedForUi(preview.text) || view.decodedTextBinaryForUi;
    view.decodedError = preview.error ?? null;
  }
  view.metadataOnly = false;
}

function streamEditKey(reference) {
  return reference ? referenceKey(normalizeReference(reference)) : "";
}

function applyStreamDraftToView(view) {
  if (!view?.reference) {
    return;
  }
  const draft = streamEditDrafts.get(streamEditKey(view.reference));
  if (!draft) {
    return;
  }
  view.decodedText = draft.decodedText;
  view.decodedLength = draft.decodedText.length;
  view.decodedTextTruncated = false;
  view.decodedTextBinaryForUi = rawTextLooksBinary(draft.decodedText);
  view.decodedTextTruncatedForUi =
    streamTextTruncatedForUi(draft.decodedText) || view.decodedTextBinaryForUi;
  view.decodedError = null;
  view.hasStreamDraft = true;
}

function activeStreamDraft() {
  return activeStreamReference ? streamEditDrafts.get(streamEditKey(activeStreamReference)) : null;
}

function canEditDecodedStream(view = activeStreamView) {
  return Boolean(
    view?.reference &&
      activeStreamMode === "decoded" &&
      typeof view.decodedText === "string" &&
      !view.decodedTextTruncated &&
      !view.decodedTextTruncatedForUi &&
      !view.decodedTextBinaryForUi,
  );
}

function updateStreamEditControls() {
  if (!elements.editDecodedStream) {
    return;
  }
  const canEdit = canEditDecodedStream();
  const draft = activeStreamDraft();
  elements.editDecodedStream.disabled = !canEdit;
  elements.editDecodedStream.textContent = draft ? "Edit Draft" : "Edit Decoded";
  elements.editDecodedStream.title = canEdit
    ? "Edit the decoded content stream in the current session draft."
    : "Load a complete decoded non-binary stream preview to edit it.";
  if (elements.streamEditPanel && !elements.streamEditPanel.hidden && !canEdit) {
    closeStreamEditPanel();
  }
}

function canRenderStreamImage(view = activeStreamView) {
  return Boolean(view?.reference && view?.image?.renderable);
}

function updateStreamImageControls() {
  if (!elements.renderStreamImage) {
    return;
  }
  const canRender = canRenderStreamImage();
  const image = activeStreamView?.image;
  elements.renderStreamImage.disabled = !canRender;
  elements.renderStreamImage.title = canRender
    ? `Render image stream (${image?.width ?? "?"} x ${image?.height ?? "?"}).`
    : image?.subtype === "Image"
      ? "This image stream uses unsupported color space, bit depth, or decode parameters."
      : "Render Image is available for Image XObject streams.";
}

function clearStreamImagePreview() {
  streamImagePreviewRequestId += 1;
  if (!elements.streamImagePreview) {
    return;
  }
  elements.streamImagePreview.hidden = true;
  elements.streamImagePreviewImage.removeAttribute("src");
  elements.streamImagePreviewImage.removeAttribute("width");
  elements.streamImagePreviewImage.removeAttribute("height");
  elements.streamImagePreviewMeta.textContent = "Rendered locally from the selected image stream.";
}

async function renderActiveStreamImage() {
  if (!currentPdfPath || !activeStreamReference || !activeStreamView) {
    return;
  }
  if (!canRenderStreamImage()) {
    showStreamViewerRecoverableError(
      activeStreamView?.image?.subtype === "Image"
        ? "This image stream cannot be rendered by the current lightweight preview path."
        : "Render Image is available only for Image XObject streams.",
    );
    return;
  }

  const requestId = ++streamImagePreviewRequestId;
  const requestPath = currentPdfPath;
  const reference = normalizeReference(activeStreamReference);
  const cacheKey = streamCacheKey(requestPath, reference);
  clearStreamViewerStatus();
  elements.streamViewerState.textContent = "Rendering image";
  elements.streamViewerState.classList.add("is-loading");

  try {
    const preview = await loadStreamImagePreviewFromCacheOrTauri(requestPath, reference, cacheKey);
    if (
      requestId !== streamImagePreviewRequestId ||
      requestPath !== currentPdfPath ||
      referenceKey(reference) !== referenceKey(activeStreamReference)
    ) {
      return;
    }
    renderStreamImagePreview(preview);
    showStreamViewerStatus("Image stream rendered locally.", "success");
  } catch (error) {
    if (
      requestId !== streamImagePreviewRequestId ||
      requestPath !== currentPdfPath ||
      referenceKey(reference) !== referenceKey(activeStreamReference)
    ) {
      return;
    }
    showStreamViewerRecoverableError(String(error));
  } finally {
    if (requestId === streamImagePreviewRequestId) {
      elements.streamViewerState.classList.remove("is-loading");
      elements.streamViewerState.textContent = "Ready";
    }
  }
}

async function loadStreamImagePreviewFromCacheOrTauri(path, reference, cacheKey) {
  const cachedPreview = streamImagePreviewCache.get(cacheKey);
  if (cachedPreview) {
    return cachedPreview;
  }
  const pendingPreview = streamImagePreviewLoading.get(cacheKey);
  if (pendingPreview) {
    return pendingPreview;
  }
  let request = null;
  request = scheduleTauriTask(
    "streamPreview",
    "render_stream_image_preview",
    {
      path,
      object: reference.object,
      generation: reference.generation,
    },
    {
      dropGroup: "streamPreview",
      coalesceKey: `streamImage:${path}:${referenceKey(reference)}`,
    },
  )
    .then((preview) => {
      if (path === currentPdfPath) {
        streamImagePreviewCache.set(cacheKey, preview);
      }
      return preview;
    })
    .finally(() => {
      if (streamImagePreviewLoading.get(cacheKey) === request) {
        streamImagePreviewLoading.delete(cacheKey);
      }
    });
  streamImagePreviewLoading.set(cacheKey, request);
  return request;
}

function renderStreamImagePreview(preview) {
  const assetUrl = convertFileSrc(preview.path);
  elements.streamImagePreviewTitle.textContent = `Image Preview - ${objectLabel(preview.reference)}`;
  elements.streamImagePreviewMeta.textContent =
    `${preview.width} x ${preview.height} px, ${preview.format.toUpperCase()}, ${preview.source}`;
  elements.streamImagePreviewImage.onerror = () => {
    showStreamViewerRecoverableError("Image preview file could not be loaded by the WebView.");
  };
  elements.streamImagePreviewImage.src = assetUrl;
  elements.streamImagePreviewImage.width = preview.width;
  elements.streamImagePreviewImage.height = preview.height;
  elements.streamImagePreview.hidden = false;
}

function openStreamEditPanel() {
  if (!canEditDecodedStream()) {
    showStreamViewerRecoverableError(
      "Decoded stream editing requires a complete, non-binary decoded preview without UI truncation.",
    );
    return;
  }
  const key = streamEditKey(activeStreamReference);
  const draft = streamEditDrafts.get(key);
  activeStreamEditKey = key;
  elements.streamEditTextarea.value = draft?.decodedText ?? activeStreamView.decodedText ?? "";
  elements.streamEditHint.textContent =
    "Apply writes this decoded content into the session memory draft and re-renders Page Preview from a temporary local PDF.";
  elements.streamEditPanel.hidden = false;
  window.setTimeout(() => elements.streamEditTextarea.focus(), 0);
}

function closeStreamEditPanel() {
  activeStreamEditKey = null;
  if (elements.streamEditPanel) {
    elements.streamEditPanel.hidden = true;
  }
}

function applyActiveStreamEdit() {
  if (!activeStreamReference || !activeStreamEditKey || !activeStreamView) {
    return;
  }
  const reference = normalizeReference(activeStreamReference);
  const decodedText = elements.streamEditTextarea.value ?? "";
  streamEditDrafts.set(activeStreamEditKey, {
    reference,
    decodedText,
  });
  activeStreamView.decodedText = decodedText;
  activeStreamView.decodedLength = decodedText.length;
  activeStreamView.decodedTextTruncated = false;
  activeStreamView.decodedTextBinaryForUi = rawTextLooksBinary(decodedText);
  activeStreamView.decodedTextTruncatedForUi =
    streamTextTruncatedForUi(decodedText) || activeStreamView.decodedTextBinaryForUi;
  activeStreamView.decodedError = null;
  activeStreamView.hasStreamDraft = true;
  closeStreamEditPanel();
  markObjectDraftsChanged();
  renderStreamViewMetadata(activeStreamView);
  renderStreamViewerContent();
  updateStreamActionButtons();
  updateStreamEditControls();
  updateStreamImageControls();
  showStreamViewerStatus("Decoded content stream draft applied in memory. Page Preview will re-render from the draft.", "info");
  if (shouldRunContentAnalysis(activeStreamView)) {
    renderDecodedText(activeStreamView.decodedText);
  }
}

function streamTextTruncatedForUi(text) {
  return typeof text === "string" && text.length > STREAM_TEXT_RENDER_LIMIT;
}

function streamTextForUi(text) {
  if (typeof text !== "string") {
    return "";
  }
  return text;
}

function decodedStreamTextForUi(text) {
  if (!rawTextLooksBinary(text)) {
    return streamTextForUi(text);
  }
  return [
    `Decoded binary stream preview. Showing escaped first ${formatBytes(STREAM_RAW_BINARY_RENDER_LIMIT)} for UI responsiveness.`,
    "This stream is likely image or binary data, not PDF content operators.",
    "Use Hex for byte-oriented inspection, or Export Decoded for the decoded bytes.",
    "",
    escapeRawBinaryPreview(text),
  ].join("\n");
}

function rawTextLooksBinary(text) {
  if (typeof text !== "string" || !text.length) {
    return false;
  }
  const sample = text.slice(0, Math.min(text.length, STREAM_RAW_BINARY_RENDER_LIMIT));
  let suspicious = 0;
  for (const character of sample) {
    const code = character.charCodeAt(0);
    if (
      character === "\uFFFD" ||
      code === 0 ||
      (code < 32 && character !== "\n" && character !== "\r" && character !== "\t")
    ) {
      suspicious += 1;
    }
  }
  return suspicious > 24 || suspicious / sample.length > 0.02;
}

function escapeRawBinaryPreview(text) {
  const sample = String(text ?? "").slice(0, STREAM_RAW_BINARY_RENDER_LIMIT);
  let output = "";
  for (const character of sample) {
    const code = character.charCodeAt(0);
    if (character === "\n" || character === "\r" || character === "\t") {
      output += character;
    } else if (character === "\uFFFD") {
      output += "\\uFFFD";
    } else if (code < 32 || code === 127) {
      output += `\\x${code.toString(16).padStart(2, "0")}`;
    } else {
      output += character;
    }
  }
  return output;
}

function clearStreamVirtualText() {
  if (virtualStreamTextState?.frame) {
    window.cancelAnimationFrame(virtualStreamTextState.frame);
  }
  virtualStreamTextState = null;
  streamVirtualTextKey = null;
  elements.streamViewerContent.onscroll = null;
  elements.streamViewerContent.classList.remove("is-virtualized", "has-highlight");
}

function setStreamViewerPlainText(text, options = {}) {
  const shouldResetScroll = options.resetScroll !== false;
  clearStreamVirtualText();
  elements.streamViewerContent.textContent = text ?? "";
  if (shouldResetScroll) {
    elements.streamViewerContent.scrollTop = 0;
  }
}

function renderStreamText(text, options = {}) {
  const value = typeof text === "string" ? text : "";
  if (streamTextTruncatedForUi(value)) {
    renderVirtualStreamText(value, options);
    return true;
  }
  setStreamViewerPlainText(value);
  return false;
}

function renderVirtualStreamText(text, options = {}) {
  const key = `${options.label ?? "Stream preview"}:${text.length}:${text.slice(0, 64)}:${text.slice(-64)}`;
  const previousScrollTop =
    streamVirtualTextKey === key ? elements.streamViewerContent.scrollTop || 0 : 0;
  clearStreamVirtualText();
  const rows = streamTextToVirtualRows(text);
  virtualStreamTextState = {
    frame: null,
    rows,
    label: options.label ?? "Stream preview",
  };
  streamVirtualTextKey = key;
  elements.streamViewerContent.onscroll = () => scheduleVirtualStreamTextRender();
  elements.streamViewerContent.classList.toggle("has-highlight", false);
  elements.streamViewerContent.classList.add("is-virtualized");
  elements.streamViewerContent.scrollTop = previousScrollTop;
  renderVirtualStreamTextRows();
}

function scheduleVirtualStreamTextRender() {
  if (!virtualStreamTextState || virtualStreamTextState.frame) {
    return;
  }
  virtualStreamTextState.frame = window.requestAnimationFrame(() => {
    if (!virtualStreamTextState) {
      return;
    }
    virtualStreamTextState.frame = null;
    renderVirtualStreamTextRows();
  });
}

function renderVirtualStreamTextRows() {
  if (!virtualStreamTextState) {
    return;
  }
  const rows = virtualStreamTextState.rows;
  const range = streamVirtualRange(elements.streamViewerContent, rows.length);
  const fragment = document.createDocumentFragment();

  if (range.before) {
    fragment.appendChild(streamVirtualSpacer(range.before));
  }

  for (let index = range.start; index < range.end; index += 1) {
    fragment.appendChild(streamVirtualTextRow(rows[index], index));
  }

  if (range.after) {
    fragment.appendChild(streamVirtualSpacer(range.after));
  }

  elements.streamViewerContent.replaceChildren(fragment);
}

function streamVirtualTextRow(text, index) {
  const row = document.createElement("div");
  row.className = "stream-virtual-row";
  row.style.height = `${streamVirtualRowHeight()}px`;
  row.style.lineHeight = `${streamVirtualRowHeight()}px`;
  row.dataset.row = String(index + 1);
  row.textContent = text.length ? text : " ";
  return row;
}

function streamTextToVirtualRows(text) {
  const rows = [];
  let lineStart = 0;
  const value = String(text ?? "");

  for (let index = 0; index <= value.length; index += 1) {
    const atEnd = index === value.length;
    const char = atEnd ? "\n" : value[index];
    if (!atEnd && char !== "\n" && char !== "\r") {
      continue;
    }

    appendStreamVirtualLine(rows, value.slice(lineStart, index));
    if (!atEnd && char === "\r" && value[index + 1] === "\n") {
      index += 1;
    }
    lineStart = index + 1;
  }

  return rows.length ? rows : [""];
}

function appendStreamVirtualLine(rows, line) {
  if (line.length <= STREAM_VIRTUAL_MAX_ROW_CHARS) {
    rows.push(line);
    return;
  }

  for (let offset = 0; offset < line.length; offset += STREAM_VIRTUAL_MAX_ROW_CHARS) {
    rows.push(line.slice(offset, offset + STREAM_VIRTUAL_MAX_ROW_CHARS));
  }
}

function mergeUniqueStrings(left = [], right = []) {
  return Array.from(new Set([...(left ?? []), ...(right ?? [])].filter(Boolean)));
}

function streamPreviewTextForMode(view, mode) {
  if (mode === "hex") return view.hexText;
  if (mode === "decoded") return view.decodedText;
  return null;
}

function streamPreviewErrorForMode(view, mode) {
  if (mode === "hex") return view.hexError;
  if (mode === "decoded") {
    return view.decodedError &&
      view.decodedError !== STREAM_DECODED_PENDING_MESSAGE &&
      view.decodedText == null
      ? view.decodedError
      : null;
  }
  return null;
}

function shouldRunContentAnalysis(view) {
  return Boolean(
    view?.decodedText != null &&
      !view.decodedTextTruncatedForUi &&
      !view.decodedTextBinaryForUi,
  );
}

function renderStreamViewMetadata(view, options = {}) {
  elements.streamViewerState.textContent = options.pendingPreview ? "Metadata ready" : "Ready";
  elements.streamViewerState.classList.toggle("is-loading", Boolean(options.pendingPreview));
  elements.streamViewerReference.textContent = objectLabel(view.reference);
  elements.streamViewerRawLength.textContent = view.rawLength ?? "-";
  elements.streamViewerByteRange.textContent = formatByteRange(view.rawRange ?? view.raw_range);
  elements.streamViewerDecodedLength.textContent = view.decodedLength ?? "Unavailable";
  elements.streamViewerFilters.textContent = view.filters?.length ? view.filters.join(" -> ") : "None";
  elements.streamViewerIssues.textContent = formatStreamIssues(view);
  elements.streamViewerError.hidden = true;
}

function formatDecodeIssues(issues) {
  return issues?.length
    ? issues.map((issue) => `${issue.filter}: ${issue.message}`).join("; ")
    : "None";
}

function formatStreamIssues(view) {
  const parts = [];
  const decodeIssues = formatDecodeIssues(view?.decodeIssues);
  if (decodeIssues !== "None") {
    parts.push(decodeIssues);
  }
  if (Array.isArray(view?.warnings) && view.warnings.length) {
    parts.push(...view.warnings);
  }
  if (view?.hexTextTruncated) {
    parts.push(`Preview is truncated to ${formatBytes(view.previewLimit ?? 0)}.`);
  }
  if (view?.decodedTextTruncated) {
    parts.push(`Decoded preview is truncated to ${formatBytes(view.previewLimit ?? 0)}.`);
  }
  if (view?.decodedTextBinaryForUi) {
    parts.push(`Decoded binary display is escaped and limited to ${formatBytes(STREAM_RAW_BINARY_RENDER_LIMIT)}.`);
  }
  const hasVirtualizedText =
    view?.hexTextTruncatedForUi ||
    (view?.decodedTextTruncatedForUi && !view?.decodedTextBinaryForUi);
  if (hasVirtualizedText) {
    parts.push(`WebView text rendering is virtualized after ${formatBytes(STREAM_TEXT_RENDER_LIMIT)}.`);
  }
  return parts.length ? parts.join("; ") : "None";
}

function setStreamMode(mode) {
  activeStreamMode = mode;
  for (const button of elements.streamModeButtons) {
    button.classList.toggle("is-active", button.dataset.mode === mode);
  }
  renderStreamViewerContent();
  loadActiveStreamPreview();
}

function renderStreamViewerContent() {
  if (!activeStreamView) {
    return;
  }
  updateStreamEditControls();

  if (activeStreamMode === "hex") {
    if (activeStreamView.hexText == null) {
      setStreamViewerPlainText(
        activeStreamView.hexError
          ? "Hex preview could not be loaded."
          : "Hex preview will load when this tab is selected.",
      );
      if (activeStreamView.hexError) {
        showStreamViewerRecoverableError(activeStreamView.hexError);
      } else {
        elements.streamViewerError.hidden = true;
      }
      return;
    }
    renderStreamText(activeStreamView.hexText, { label: "Hex" });
    if (activeStreamView.hexTextTruncated || activeStreamView.hexTextTruncatedForUi) {
      showStreamViewerRecoverableError(
        streamPreviewTruncationMessage("Hex", activeStreamView.hexTextTruncated, activeStreamView.hexTextTruncatedForUi),
      );
    } else {
      elements.streamViewerError.hidden = true;
    }
    return;
  }

  if (activeStreamMode === "decoded") {
    if (activeStreamView.decodedText != null) {
      if (activeStreamView.decodedTextBinaryForUi) {
        renderStreamText(decodedStreamTextForUi(activeStreamView.decodedText), { label: "Decoded" });
      } else {
        renderDecodedText(activeStreamView.decodedText);
      }
      if (activeStreamView.decodedTextTruncated || activeStreamView.decodedTextTruncatedForUi) {
        showStreamViewerRecoverableError(
          streamPreviewTruncationMessage("Decoded", activeStreamView.decodedTextTruncated, activeStreamView.decodedTextTruncatedForUi, "Export Decoded"),
        );
      } else {
          elements.streamViewerError.hidden = true;
      }
    } else {
      setStreamViewerPlainText("");
      elements.streamViewerError.textContent =
        activeStreamView.decodedError && activeStreamView.decodedError !== STREAM_DECODED_PENDING_MESSAGE
          ? activeStreamView.decodedError
          : "Decoded preview will load when this tab is selected.";
      elements.streamViewerError.hidden = false;
    }
    return;
  }
}

function streamPreviewTruncationMessage(label, backendTruncated, uiTruncated, exportLabel) {
  const pieces = [];
  if (backendTruncated) {
    pieces.push(`${label} preview is truncated to ${formatBytes(activeStreamView.previewLimit ?? 0)}.`);
  }
  if (label === "Decoded" && activeStreamView?.decodedTextBinaryForUi) {
    pieces.push(`${label} binary display is escaped and limited to ${formatBytes(STREAM_RAW_BINARY_RENDER_LIMIT)} to keep the GUI responsive.`);
  } else if (uiTruncated) {
    pieces.push(`${label} text display is virtualized after ${formatBytes(STREAM_TEXT_RENDER_LIMIT)}; only visible rows are mounted to keep the GUI responsive.`);
  }
  if (exportLabel) {
    pieces.push(`Use ${exportLabel} for the full stream.`);
  }
  return pieces.join(" ");
}

function updateStreamActionButtons() {
  const hasStream = Boolean(activeStreamView && activeStreamReference);
  const hasDecodedText = Boolean(activeStreamView?.decodedText != null);
  const hasDecodedBytes = hasStream && activeStreamView.decodedLength != null;
  const canExportDecoded =
    hasStream && activeStreamView?.canExportDecoded !== false && hasDecodedBytes;

  elements.copyDecodedStream.disabled = !hasDecodedText;
  elements.exportDecodedStream.disabled = !canExportDecoded;
  updateStreamImageControls();
}

function showStreamViewerStatus(message, kind = "success") {
  elements.streamViewerStatus.textContent = message;
  elements.streamViewerStatus.dataset.kind = kind;
  elements.streamViewerStatus.hidden = false;
}

function clearStreamViewerStatus() {
  elements.streamViewerStatus.textContent = "";
  elements.streamViewerStatus.hidden = true;
  delete elements.streamViewerStatus.dataset.kind;
}

function showStreamViewerRecoverableError(message) {
  elements.streamViewerError.textContent = message;
  elements.streamViewerError.hidden = false;
}

async function copyStreamText(mode) {
  if (!activeStreamView) {
    return;
  }

  clearStreamViewerStatus();
  const text = mode === "decoded" ? activeStreamView.decodedText : null;
  if (text == null) {
    showStreamViewerRecoverableError("Decoded stream text is unavailable.");
    return;
  }

  try {
    await navigator.clipboard.writeText(text);
    showStreamViewerStatus("Decoded stream text copied.");
  } catch (error) {
    showStreamViewerRecoverableError(`Could not copy stream text: ${error}`);
  }
}

async function exportStream(mode) {
  if (!currentPdfPath || !activeStreamReference) {
    return;
  }

  clearStreamViewerStatus();
  const defaultPath = streamExportFileName(mode);
  const selected = await save({
    defaultPath,
    filters: [
      {
        name: mode === "decoded" ? "Decoded stream" : "Raw stream",
        extensions: mode === "decoded" ? ["txt", "bin"] : ["bin"],
      },
    ],
  });

  if (typeof selected !== "string") {
    return;
  }

  try {
    const bytesWritten = await invoke("export_stream", {
      path: currentPdfPath,
      object: activeStreamReference.object,
      generation: activeStreamReference.generation,
      outputPath: selected,
      mode,
    });
    showStreamViewerStatus(
      `${mode === "decoded" ? "Decoded" : "Raw"} stream exported (${formatBytes(bytesWritten)}).`,
    );
  } catch (error) {
    showStreamViewerRecoverableError(String(error));
  }
}

function streamExportFileName(mode) {
  const reference = activeStreamReference ? objectLabel(activeStreamReference).replaceAll(" ", "-") : "stream";
  const suffix = mode === "decoded" ? "decoded.txt" : "raw.bin";
  return `${reference}-${suffix}`;
}

function renderDecodedText(text) {
  const tokens = normalizedContentTokens(activeContentTokens);
  const operators = normalizedContentOperators(activeContentOperators);
  if (streamTextTruncatedForUi(text)) {
    renderStreamText(text, { label: "Decoded" });
    return;
  }
  if (!tokens.length) {
    setStreamViewerPlainText(text);
    return;
  }

  clearStreamVirtualText();
  elements.streamViewerContent.scrollTop = 0;
  elements.streamViewerContent.replaceChildren(
    ...buildHighlightedDecodedNodes(text, tokens, operators),
  );
  elements.streamViewerContent.classList.toggle("has-highlight", true);
}

function buildHighlightedDecodedNodes(text, tokens, operators = []) {
  const boundaries = new Set([0, text.length]);
  const tokenRanges = [];
  const operatorRangeKeys = new Set(
    operators.map((operator) => `${operator.byte_range.start}:${operator.byte_range.end}`),
  );

  for (const token of tokens) {
    const range = stringRangeForByteRange(text, token.byte_range);
    if (!range) {
      continue;
    }

    const byteRange = normalizedRange(token.byte_range);
    const isOperator = byteRange
      ? operatorRangeKeys.has(`${byteRange.start}:${byteRange.end}`)
      : false;
    tokenRanges.push({ ...range, kind: isOperator ? "operator" : token.kind });
    boundaries.add(range.start);
    boundaries.add(range.end);
  }

  const sortedBoundaries = Array.from(boundaries)
    .filter((boundary) => boundary >= 0 && boundary <= text.length)
    .sort((left, right) => left - right);
  const nodes = [];

  for (let index = 0; index < sortedBoundaries.length - 1; index += 1) {
    const start = sortedBoundaries[index];
    const end = sortedBoundaries[index + 1];
    if (end <= start) {
      continue;
    }

    const segment = text.slice(start, end);
    const token = tokenRanges.find((range) => range.start <= start && range.end >= end);

    if (!token) {
      nodes.push(document.createTextNode(segment));
      continue;
    }

    const element = document.createElement("span");
    element.className = tokenClassName(token?.kind);
    element.textContent = segment;
    nodes.push(element);
  }

  return nodes;
}

function normalizedContentTokens(tokens) {
  if (!Array.isArray(tokens)) {
    return [];
  }

  return tokens
    .filter((token) => token?.byte_range || token?.byteRange)
    .map((token) => ({
      kind: token.kind,
      byte_range: token.byte_range ?? token.byteRange,
    }));
}

function normalizedContentOperators(operators) {
  if (!Array.isArray(operators)) {
    return [];
  }

  return operators
    .filter((operator) => operator?.byte_range || operator?.byteRange)
    .map((operator) => ({
      byte_range: normalizedRange(operator.byte_range ?? operator.byteRange),
    }))
    .filter((operator) => operator.byte_range);
}

function stringRangeForByteRange(text, byteRange) {
  const range = normalizedRange(byteRange);
  if (!range) {
    return null;
  }

  const start = byteOffsetToStringIndex(text, range.start);
  const end = byteOffsetToStringIndex(text, range.end);
  if (start == null || end == null || end <= start) {
    return null;
  }

  return { start, end };
}

function tokenClassName(kind) {
  const normalized = String(kind ?? "other").replaceAll("_", "-");
  return `content-token token-${normalized}`;
}

function normalizedRange(range) {
  if (!range || !Number.isFinite(range.start) || !Number.isFinite(range.end)) {
    return null;
  }

  const start = Math.max(0, Math.trunc(range.start));
  const end = Math.max(start, Math.trunc(range.end));
  return end > start ? { start, end } : null;
}

function byteOffsetToStringIndex(text, byteOffset) {
  if (byteOffset <= 0) {
    return 0;
  }

  let bytesSeen = 0;
  let index = 0;
  for (const character of text) {
    if (bytesSeen >= byteOffset) {
      return index;
    }

    const codePoint = character.codePointAt(0);
    bytesSeen += utf8ByteLength(codePoint);
    index += character.length;

    if (bytesSeen >= byteOffset) {
      return index;
    }
  }

  return byteOffset <= bytesSeen ? index : null;
}

function utf8ByteLength(codePoint) {
  if (codePoint <= 0x7f) {
    return 1;
  }
  if (codePoint <= 0x7ff) {
    return 2;
  }
  if (codePoint <= 0xffff) {
    return 3;
  }
  return 4;
}

async function loadContentAnalysis(reference) {
  if (!currentPdfPath) {
    return;
  }

  const requestId = ++contentAnalysisRequestId;

  try {
    const view = await scheduleTauriTask(
      "streamPreview",
      "view_content_stream",
      {
        path: currentPdfPath,
        object: reference.object,
        generation: reference.generation,
      },
      {
        dropGroup: "streamPreview",
        coalesceKey: `contentStream:${currentPdfPath}:${referenceKey(reference)}`,
      },
    );
    if (requestId !== contentAnalysisRequestId) {
      return;
    }
    activeContentTokens = view.tokens ?? [];
    activeContentOperators = view.operators ?? [];
    renderStreamViewerContent();
  } catch (error) {
    if (requestId !== contentAnalysisRequestId) {
      return;
    }
    activeContentTokens = [];
    activeContentOperators = [];
    renderStreamViewerContent();
  }
}

function tableCell(text, className) {
  const cell = document.createElement("td");
  if (className) {
    cell.className = className;
  }
  cell.textContent = text || "-";
  cell.title = text || "-";
  return cell;
}

async function loadPdf(path) {
  if (!path) {
    return;
  }

  const openGeneration = ++pdfOpenGeneration;
  pageListRequestId += 1;
  currentPdfPath = null;
  currentOpenMode = "loading";
  updateDocumentInteractionState();
  resetObjectEditDrafts();
  clearDocumentWorkspaceTabs();
  if (!path.toLowerCase().endsWith(".pdf")) {
    currentPdfPath = null;
    resetSummary();
    resetNavigationState();
    clearInspector();
    clearPagePreview();
    elements.objectTree.textContent = "Open a PDF to populate the structure.";
    elements.objectTree.classList.add("empty-tree");
    elements.treeCount.textContent = "0";
    showError("Please choose or drop a local .pdf file.");
    return;
  }

  clearError();
  clearDocumentWorkspaceTabs();
  resetNavigationState();
  clearPagePreview("Loading page list...");
  clearInspector();
  setDocumentLoadingMessage(path);
  setLoading(true);
  const perf = perfMark("open_pdf", fileNameFromPath(path));
  try {
    const summary = await invoke("open_pdf", { path });
    if (openGeneration !== pdfOpenGeneration) {
      perfDone(perf, "stale");
      return;
    }
    perfDone(perf);
    renderSummary(summary);
    updateWindowSubtitle(summary.path ?? path);
    rememberRecentFile(path);
  } catch (error) {
    if (openGeneration !== pdfOpenGeneration) {
      perfDone(perf, "stale-error");
      return;
    }
    perfDone(perf, "error");
    currentPdfPath = null;
    currentOpenMode = "none";
    treeButtonsByReference = new Map();
    resetSummary();
    clearInspector();
    clearPagePreview();
    showError(String(error));
    elements.objectTree.textContent = "Open a PDF to populate the structure.";
    elements.objectTree.classList.add("empty-tree");
    elements.treeCount.textContent = "0";
  } finally {
    if (openGeneration === pdfOpenGeneration) {
      setLoading(false);
      updateDocumentInteractionState();
    }
  }
}

async function choosePdf() {
  clearError();
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [
      {
        name: "PDF",
        extensions: ["pdf"],
      },
    ],
  });

  if (typeof selected === "string") {
    await loadPdf(selected);
  }
}

for (const button of elements.openButtons) {
  button.addEventListener("click", choosePdf);
}

elements.windowDragRegion.addEventListener("mousedown", startWindowDrag);
elements.windowDragRegion.addEventListener("dblclick", (event) => {
  if (event.target?.closest?.("[data-no-drag]")) {
    return;
  }
  event.preventDefault();
  toggleWindowMaximize();
});
elements.windowMinimize.addEventListener("click", minimizeWindow);
elements.windowMaximize.addEventListener("click", toggleWindowMaximize);
elements.windowClose.addEventListener("click", closeWindow);

elements.clearRecentFiles.addEventListener("click", clearRecentFiles);

elements.fileMenuButton.addEventListener("click", (event) => {
  event.stopPropagation();
  setAppearancePanelOpen(false);
  setFileMenuOpen(!fileMenuOpen);
});
elements.fileMenu.addEventListener("click", (event) => {
  event.stopPropagation();
});
elements.fileMenuSave.addEventListener("click", saveModifiedPdfFromFileMenu);
elements.fileMenuSaveAs.addEventListener("click", saveModifiedPdfAsFromFileMenu);
elements.appearanceToggle.addEventListener("click", (event) => {
  event.stopPropagation();
  setFileMenuOpen(false);
  setAppearancePanelOpen(!appearancePanelOpen);
});
elements.appearancePanel.addEventListener("click", (event) => {
  event.stopPropagation();
});
elements.appearanceReset.addEventListener("click", resetAppearanceSettings);
elements.appearanceResetColumns.addEventListener("click", resetTreeColumnWidths);
elements.appearanceTheme.addEventListener("change", () => updateAppearanceSetting("theme", elements.appearanceTheme.value));
elements.uiFontFamily.addEventListener("change", () => updateAppearanceSetting("uiFontFamily", elements.uiFontFamily.value));
elements.uiFontSize.addEventListener("change", () => updateAppearanceSetting("uiFontSize", elements.uiFontSize.value));
elements.monoFontFamily.addEventListener("change", () => updateAppearanceSetting("monoFontFamily", elements.monoFontFamily.value));
elements.monoFontSize.addEventListener("change", () => updateAppearanceSetting("monoFontSize", elements.monoFontSize.value));
for (const resizer of elements.treeColumnResizers) {
  resizer.addEventListener("pointerdown", startTreeColumnResize);
}

for (const button of elements.navTabButtons) {
  button.addEventListener("click", () => setActiveWorkspaceTab(button.dataset.workspaceTab));
}

elements.pageMetadataFloat.addEventListener("click", () => {
  setPageMetadataFloating(!isPageMetadataFloating);
});

elements.objectTreeSearch.addEventListener("input", () => {
  objectTreeSearchQuery = elements.objectTreeSearch.value ?? "";
  elements.objectTree.scrollTop = 0;
  refreshObjectTreeSearchRows();
  renderVirtualObjectTree();
});

elements.objectTreeSearch.addEventListener("keydown", (event) => {
  if (event.key !== "Escape" || !objectTreeSearchQuery) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  clearObjectTreeSearch();
  renderVirtualObjectTree();
});

elements.pageReferenceSearch.addEventListener("input", () => {
  pageReferenceSearchQuery = elements.pageReferenceSearch.value ?? "";
  renderFilteredPageObjectLinks();
});

elements.acroformSearch.addEventListener("input", () => {
  acroFormSearchQuery = elements.acroformSearch.value ?? "";
  renderAcroForm(activeAcroForm);
});
elements.annotsSearch.addEventListener("input", () => {
  annotsSearchQuery = elements.annotsSearch.value ?? "";
  renderAnnotations(activeAnnotations);
});
elements.revertObjectEdits.addEventListener("click", revertCurrentObjectEdits);
elements.revertAllEdits.addEventListener("click", revertAllObjectEdits);
elements.saveModifiedPdf.addEventListener("click", saveModifiedPdfAs);
elements.saveEditsAndRerender.addEventListener("click", () => saveModifiedPdfAs({
  reRender: true,
  reRenderPageNumber: activePageNumber(),
}));

elements.workspaceTabOverflowButton.addEventListener("click", (event) => {
  event.stopPropagation();
  setWorkspaceTabOverflowOpen(!workspaceTabOverflowOpen);
});

document.addEventListener("click", (event) => {
  if (
    fileMenuOpen &&
    !elements.fileMenuButton.contains(event.target) &&
    !elements.fileMenu.contains(event.target)
  ) {
    setFileMenuOpen(false);
  }
  if (
    appearancePanelOpen &&
    !elements.appearanceToggle.contains(event.target) &&
    !elements.appearancePanel.contains(event.target)
  ) {
    setAppearancePanelOpen(false);
  }
  if (
    !workspaceTabOverflowOpen ||
    elements.workspaceTabOverflowButton.contains(event.target) ||
    elements.workspaceTabOverflowMenu.contains(event.target)
  ) {
    return;
  }
  setWorkspaceTabOverflowOpen(false);
});

document.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    setAppearancePanelOpen(false);
    setFileMenuOpen(false);
    setWorkspaceTabOverflowOpen(false);
    return;
  }
  if (event.key === "F5" || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "r")) {
    event.preventDefault();
    return;
  }
  if ((event.ctrlKey || event.metaKey) && !event.shiftKey && event.key.toLowerCase() === "w") {
    event.preventDefault();
    closeActiveWorkspaceTab();
    return;
  }
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
    event.preventDefault();
    if (event.shiftKey) {
      saveModifiedPdfAsFromFileMenu();
    } else {
      saveModifiedPdfFromFileMenu();
    }
  }
});

window.addEventListener("resize", () => {
  scheduleWorkspaceTabOverflowUpdate();
  applyPagePreviewZoom();
  schedulePageMetadataFloatingLayoutSync();
  updateWindowMaximizeButton();
});

const systemThemeQuery = window.matchMedia?.("(prefers-color-scheme: dark)");
const handleSystemThemeChange = () => {
  if (appearanceSettings?.theme === "system") {
    applyAppearanceSettings();
  }
};
if (systemThemeQuery?.addEventListener) {
  systemThemeQuery.addEventListener("change", handleSystemThemeChange);
} else if (systemThemeQuery?.addListener) {
  systemThemeQuery.addListener(handleSystemThemeChange);
}

for (const button of elements.streamModeButtons) {
  button.addEventListener("click", () => setStreamMode(button.dataset.mode));
}
elements.openStreamDetails.addEventListener("click", () => {
  if (inspectedStreamReference) {
    openStreamWorkspaceTab(inspectedStreamReference);
  }
});
elements.copyDecodedStream.addEventListener("click", () => copyStreamText("decoded"));
elements.exportDecodedStream.addEventListener("click", () => exportStream("decoded"));
elements.editDecodedStream.addEventListener("click", openStreamEditPanel);
elements.renderStreamImage.addEventListener("click", renderActiveStreamImage);
elements.closeStreamImagePreview.addEventListener("click", clearStreamImagePreview);
elements.applyStreamEdit.addEventListener("click", applyActiveStreamEdit);
elements.cancelStreamEdit.addEventListener("click", closeStreamEditPanel);
elements.pageZoomOut.addEventListener("click", () => adjustPagePreviewZoom(-PAGE_PREVIEW_ZOOM_STEP));
elements.pageZoomIn.addEventListener("click", () => adjustPagePreviewZoom(PAGE_PREVIEW_ZOOM_STEP));
elements.pageZoomReset.addEventListener("click", resetPagePreviewZoom);
elements.pagePreviewStage.addEventListener("click", handlePagePreviewClick);
elements.pagePreviewViewport.addEventListener("wheel", handlePagePreviewWheel, { passive: false });
elements.pageSelectedObjectOpen.addEventListener("click", () => {
  if (selectedPageObject?.reference) {
    openObjectWorkspaceTab(selectedPageObject.reference);
  }
});
elements.pageObjectOverlay.addEventListener("click", () => {
  if (selectedPageObject) {
    const row = elements.pageObjectsList.querySelector(`[data-page-object-id="${selectedPageObject.id}"]`);
    selectPageObject(selectedPageObject, row);
  }
});

getCurrentWindow().onDragDropEvent((event) => {
  const payload = event.payload;
  if (payload.type === "over") {
    elements.dropZone.classList.add("is-dragging");
    return;
  }

  elements.dropZone.classList.remove("is-dragging");
  if (payload.type !== "drop") {
    return;
  }

  const [path] = payload.paths ?? [];
  loadPdf(path);
});

initializeWorkspaceTabs();
updateWindowSubtitle();
updateWindowMaximizeButton();
updateFileMenuState();
updatePageMetadataFloatButton();
updateDocumentInteractionState();
loadAppearanceSettings();
loadTreeColumnWidths();
loadRecentFiles();
