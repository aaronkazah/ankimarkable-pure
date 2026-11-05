import QtQuick
import device.global
import com.remarkable
import xofm.libs.library
import xofm.libs.explorer
import xofm.libs.retail
import xofm.libs.devicescreen
import xofm.libs.peninput
import xofm.libs.toolbar

FocusScope {
    id: root
    focus: visible
    property var editor: null
    property var documentTelemetry: null
    property string cardId: ""
    property bool ready: false
    property bool changing: false
    property bool freshResponse: false
    property string expectedPageId: ""
    property var drawingScene: null
    property var inputSurface: null
    property var appWindow: null
    property var surfaceLayer: null
    property var coverCard: null
    property var documentListener: null
    property string previousDocumentId: ""
    property var toolbarPositioner: null
    property var writingToolbar: null
    property int previousToolbarPreference: 0
    signal notebookCreated(string bookId)
    signal sheetCreated(string cardId, string bookId, string pageId)
    signal returnToDecks
    signal error(string message)

    onVisibleChanged: {
        if (surfaceLayer) surfaceLayer.visible = visible && ready;
        if (visible)
            Qt.callLater(activateInput);
    }
    onWidthChanged: Qt.callLater(refreshSurface)
    onHeightChanged: Qt.callLater(refreshSurface)
    onYChanged: Qt.callLater(refreshSurface)

    Connections {
        target: root.editor
        function onIsLoadingChanged() {
            root.activateInput();
        }
        function onCurrentPageIdChanged() {
            root.activateInput();
        }
    }

    Connections {
        target: root.writingToolbar
        function onVisibleChanged() { root.activateInput(); }
        function onExpandedChanged() { root.activateInput(); }
        function onShownToolsCountChanged() { root.activateInput(); }
        function onPositionChanged() { root.activateInput(); }
        function onInnerHeightChanged() { root.activateInput(); }
    }

    Component {
        id: surfaceLayerComponent
        Item {}
    }

    Component.onDestruction: {
        closeDocument();
        if (editor) editor.destroy();
        if (surfaceLayer) surfaceLayer.destroy();
    }

    function warmUp() {
        if (editor) return;
        const request = new XMLHttpRequest();
        request.open("GET", "file:///home/root/.ankimarkable-pure/native-notes.json", false);
        request.send();
        let saved;
        try { saved = JSON.parse(request.responseText); } catch (error) { return; }
        const book = saved.book_id ? Library.entryForId(saved.book_id) : null;
        if (!book || !saved.page_id || book.pageForId(saved.page_id) < 0) return;
        openCard({card_id: "warmup", book_id: saved.book_id, page_id: saved.page_id});
    }

    function prepareCard() {
        ready = false;
        expectedPageId = "";
        coverCard = null;
        if (surfaceLayer) surfaceLayer.visible = false;
    }

    function refreshSurface() {
        if (visible && inputSurface && drawingScene?.tileManager) {
            if (editor.parent === surfaceLayer) {
                const origin = root.mapToItem(surfaceLayer, 0, 0);
                editor.x = origin.x;
                editor.y = origin.y;
            }
            inputSurface.updateTransform();
            inputSurface.manager?.updateRegions();
        }
    }

    function activateInput() {
        if (ready || changing || !visible || !editor || !expectedPageId || String(editor.currentPageId) !== expectedPageId || editor.isLoading || !drawingScene?.controller || !drawingScene?.viewport || !inputSurface)
            return;
        if (coverCard) {
            const card = coverCard;
            coverCard = null;
            expectedPageId = "";
            writeCover();
            addWritingPage(editor.document, card.card_id);
            return;
        }
        if (freshResponse) {
            freshResponse = false;
            drawingScene.controller.clearLines();
            drawingScene.controller.clearRootDocument();
            drawingScene.resetViewport();
            drawingScene.controller.requestDeferredCompleted();
            Qt.callLater(root.activateInput);
            return;
        }
        // AppLoad's window wrapper excludes native pen input. Keep the editor
        // beside that window, retaining its native toolbar exclusion regions.
        editor.parent = surfaceLayer;
        surfaceLayer.visible = visible;
        editor.forceActiveFocus();
        drawingScene.forceActiveFocus();
        refreshSurface();
        ready = drawingScene.activeFocus && writingToolbar?.visible && writingToolbar.expanded
            && writingToolbar.shownToolsCount > 0
            && writingToolbar.position === ToolbarPositioner.Position.Top
            && writingToolbar.innerHeight > 0;
        if (ready) Qt.callLater(refreshSurface);
    }

    function hostContext() {
        let top = root;
        while (top.parent)
            top = top.parent;
        const queue = [top];
        for (let i = 0; i < queue.length; i++) {
            const item = queue[i];
            if (item !== root && item.penInput !== undefined && item.toolbarProvider !== undefined && item.ghostBuster !== undefined)
                return item;
            for (let child of item.children || [])
                queue.push(child);
        }
        return null;
    }

    function createEditor() {
        const host = hostContext();
        if (!host) {
            root.error("The native editor context is unavailable. Reopen through AppLoad.");
            return false;
        }
        appWindow = root.parent;
        while (appWindow && appWindow.windowCanvas === undefined)
            appWindow = appWindow.parent;
        if (!appWindow) {
            root.error("The AppLoad window is unavailable. Reopen through AppLoad.");
            return false;
        }
        surfaceLayer = surfaceLayerComponent.createObject(appWindow.parent, {
            width: Qt.binding(() => appWindow.parent.width),
            height: Qt.binding(() => appWindow.parent.height),
            z: Qt.binding(() => appWindow.z + 1),
            visible: false
        });
        documentListener = host.documentViewListener;
        previousDocumentId = documentListener.openedDocumentId;
        toolbarPositioner = host.toolbarProvider.positioner;
        previousToolbarPreference = toolbarPositioner.preference;
        toolbarPositioner.preference = ToolbarPositioner.Position.Top;
        const component = Qt.createComponent("qrc:/qml/device/view/documentview/DocumentView.qml");
        if (component.status !== Component.Ready) {
            root.error(component.errorString());
            return false;
        }
        const properties = {
            width: root.width,
            height: root.height,
            visible: true,
            focus: true,
            orientation: host.orientation,
            notificationQueue: host.notificationQueue,
            penClose: Qt.binding(() => PenClosenessMonitor.penWasClose),
            textColumn: host.textColumn,
            penInput: host.penInput,
            toolbarConfiguration: host.toolbarConfiguration,
            toolbarProvider: host.toolbarProvider,
            retailSettings: RetailDemo.settings,
            epubSettings: host.epubSettings,
            shareAndConvert: host.shareAndConvert,
            searchIndexStatus: host.searchIndexStatus,
            analytics: host.analytics,
            ghostBuster: host.ghostBuster,
            colorProfile: host.colorProfile,
            extraViewGestures: host.extraViewGestures,
            movePageSelection: host.pageSelection,
            windowNavigator: host.windowNavigator
        };
        if (host.toolSet !== undefined) {
            properties.toolSet = host.toolSet;
            properties.deviceSceneViewBehavior = host.deviceSceneViewBehavior;
            properties.documentViewListener = host.documentViewListener;
            properties.documentOrientation = host.documentOrientation;
        } else {
            documentTelemetry = Qt.createQmlObject(
                'import QtQuick; import com.remarkable.devicetelemetry; '
                + 'import com.remarkable.telemetry as Telemetry; '
                + 'DocumentTelemetry { dispatcher: Telemetry.Dispatcher }', root);
            documentTelemetry.document = Qt.binding(() => root.editor?.document ?? null);
            properties.documentTelemetry = documentTelemetry;
        }
        editor = component.createObject(root, properties);
        if (!editor) {
            root.error(component.errorString());
            return false;
        }
        editor.width = Qt.binding(() => root.width);
        editor.height = Qt.binding(() => root.height);
        editor.documentClosed.connect(() => {
            ready = false;
            if (!changing)
                root.returnToDecks();
        });
        editor.documentChanged.connect(() => {
            documentListener.openedDocumentId = editor.document ? editor.document.id : previousDocumentId;
        });
        editor.documentFailedToLoad.connect(message => root.error(message));
        const items = [editor];
        for (let i = 0; i < items.length; i++) {
            const item = items[i];
            if (item.strokeHandler !== undefined && item.tileManager !== undefined && item.viewport !== undefined) {
                drawingScene = item;
            }
            if (typeof item.updateTransform === "function" && typeof item.setTransform === "function" && item.manager !== undefined)
                inputSurface = item;
            if (item.primaryPen !== undefined && item.container !== undefined) {
                writingToolbar = item;
                item.container = surfaceLayer;
            }
            for (let child of item.children || [])
                items.push(child);
        }
        return true;
    }

    function openCard(card) {
        if (cardId === card.card_id && editor?.document && ready)
            return true;
        savePage();
        ready = false;
        expectedPageId = "";
        if (!editor && !createEditor())
            return false;
        toolbarPositioner.preference = ToolbarPositioner.Position.Top;
        writingToolbar.expanded = true;
        let book = card.book_id ? Library.entryForId(card.book_id) : null;
        if (card.book_id && !book) {
            root.error("Restore the Anki notebook from Trash to use your saved writing.");
            return false;
        }
        if (!book) {
            const parentId = NavigationManager.activeContext.explorer.rootId().toString();
            const bookId = LibraryController.createDocument(parentId, "Anki");
            book = Library.entryForId(bookId);
            if (book) {
                LibraryController.setOrientation(book.id, Qt.Vertical);
                DocumentController.setTemplateForPage(book.id, 0, "Blank", DeviceScreenInfo.paperSize);
                LibraryController.setCoverPageNumber(book.id, 0);
                root.notebookCreated(book.id.toString());
            }
        }
        if (!book) {
            root.error("Could not open the answer notebook.");
            return false;
        }
        cardId = card.card_id;
        freshResponse = true;
        const existing = card.page_id ? book.pageForId(card.page_id) : -1;
        if (existing >= 0) {
            openPage(book, existing);
        } else if (book.pageCount === 1) {
            coverCard = card;
            openPage(book, 0);
        } else {
            addWritingPage(book, card.card_id);
        }
        return true;
    }

    function openPage(book, index) {
        const sameBook = editor.document && String(editor.document.id) === String(book.id);
        expectedPageId = book.idForPage(index).toString();
        if (sameBook) {
            if (String(editor.currentPageId) !== expectedPageId)
                editor.openPage(index);
        } else {
            changing = true;
            editor.openDocumentOnPage(book, index);
            changing = false;
        }
        Qt.callLater(root.activateInput);
    }

    function writeCover() {
        if (drawingScene.controller.rootDocumentLength === 0) {
            editor.enterTextMode(false);
            drawingScene.controller.replaceText("ANKI\n\nFLASHCARDS + HANDWRITING\n\nOpen Anki from Apps to review and sync.\n\nThe writing area clears for each new attempt.");
            editor.exitTextMode();
            drawingScene.controller.requestDeferredCompleted();
        }
        LibraryController.setCoverPageNumber(editor.document.id, 0);
    }

    function addWritingPage(book, id) {
        expectedPageId = "";
        const index = book.pageCount;
        DocumentController.addPageWithTemplateAndPageSize(book.id, index, "Blank", DeviceScreenInfo.paperSize, () => {
            if (cardId !== id)
                return;
            openPage(book, index);
            root.sheetCreated(id, book.id.toString(), book.idForPage(index).toString());
        });
    }

    function hideResponse() {
        savePage();
        ready = false;
        expectedPageId = "";
        if (surfaceLayer) surfaceLayer.visible = false;
    }

    function closeDocument() {
        ready = false;
        freshResponse = false;
        coverCard = null;
        expectedPageId = "";
        changing = true;
        if (editor?.document) {
            savePage();
            editor.close();
        }
        if (documentListener)
            documentListener.openedDocumentId = previousDocumentId;
        if (toolbarPositioner)
            toolbarPositioner.preference = previousToolbarPreference;
        changing = false;
        cardId = "";
        if (surfaceLayer) surfaceLayer.visible = false;
    }

    function savePage() {
        if (!editor?.document || !drawingScene?.controller)
            return;
        drawingScene.controller.requestDeferredCompleted();
    }

}
