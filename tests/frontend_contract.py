from pathlib import Path
import os
import shutil
import subprocess
import tempfile
from contextlib import ExitStack
from workflow_backend import workflow_server

ROOT = Path(__file__).resolve().parents[1]
SDK = Path("/opt/codex/sysroots")
TARGET = SDK / "cortexa55-remarkable-linux"
HOST = SDK / "aarch64-codexsdk-linux"

with ExitStack() as stack:
    directory = stack.enter_context(tempfile.TemporaryDirectory(prefix="ankipure-frontend-"))
    stage = Path(directory)
    binary = os.environ.get("ANKIMARKABLE_TEST_BINARY")
    endpoint = stack.enter_context(workflow_server(binary, stage)) if binary else ""
    shutil.copy(ROOT / "frontend/Main.qml", stage / "Main.qml")
    shutil.copy(ROOT / "tests/tst_navigation.qml", stage / "tst_navigation.qml")
    shutil.copy(ROOT / "frontend/NativeEditor.qml", stage / "ProductionNativeEditor.qml")

    def module(name: str, declarations: dict[str, str]) -> None:
        folder = stage / "imports" / name.replace(".", "/")
        folder.mkdir(parents=True)
        entries = [f"module {name}"]
        if not declarations:
            declarations = {"Placeholder": "import QtQuick\nQtObject {}\n"}
        for declaration, source in declarations.items():
            kind = declaration.removeprefix("singleton ")
            entries.append(f"{declaration} 1.0 {kind}.qml")
            (folder / f"{kind}.qml").write_text(source)
        (folder / "qmldir").write_text("\n".join(entries) + "\n")

    module("net.asivery.AppLoad", {"AppLoad": """import QtQuick
QtObject {
    property string applicationID
    property var lastRequest: null
    property bool live: false
    signal messageReceived(int type, string contents)
    function sendMessage(type, contents) {
        lastRequest = JSON.parse(contents);
        if (!live) return;
        const xhr = new XMLHttpRequest();
        xhr.open("POST", "WORKFLOW_ENDPOINT", true);
        xhr.onreadystatechange = function() {
            if (xhr.readyState === XMLHttpRequest.DONE) {
                if (xhr.status !== 200) throw new Error(xhr.responseText);
                messageReceived(101, xhr.responseText);
            }
        };
        xhr.send(contents);
    }
}
""".replace("WORKFLOW_ENDPOINT", endpoint), "singleton AppLoadLauncher": """pragma Singleton
import QtQuick
QtObject {
    property var launches: []
    function launchApplication(id, args, environment, detached) {
        launches = launches.concat([id]);
    }
}
"""})
    module("xofm.libs.epaper", {"ScreenModeItem": """import QtQuick
Item { enum Mode { UI, Animation } property int mode }
"""})
    for name in ("device.global", "com.remarkable",
                 "xofm.libs.retail",
                 "xofm.libs.peninput"):
        module(name, {})
    module("xofm.libs.toolbar", {"ToolbarPositioner": """import QtQuick
QtObject { enum Position { Left, Right, Top, Bottom } }
"""})
    module("xofm.libs.explorer", {"singleton NavigationManager": """pragma Singleton
import QtQuick
QtObject { property var activeContext: ({explorer:{rootId:() => ""}}) }
"""})
    module("xofm.libs.devicescreen", {"singleton DeviceScreenInfo": """pragma Singleton
import QtQuick
QtObject { property size paperSize: Qt.size(1404,1872) }
"""})
    module("xofm.libs.library", {"singleton Library": """pragma Singleton
import QtQuick
QtObject {
    property QtObject book: QtObject {
        property string id: "book"
        property var pages: ["cover"]
        readonly property int pageCount: pages.length
        function idForPage(index) { return pages[index]; }
        function pageForId(id) { return pages.indexOf(id); }
    }
    function entryForId(id) { return id === "book" ? book : null; }
}
""", "singleton DocumentController": """pragma Singleton
import QtQuick
QtObject {
    function setTemplateForPage(bookId, index, template, size) {}
    function addPageWithTemplateAndPageSize(bookId, index, template, size, done) {
        Library.book.pages = Library.book.pages.concat(["page-" + index]);
        done();
    }
}
""", "singleton LibraryController": """pragma Singleton
import QtQuick
QtObject {
    property var lastCover: null
    property int created: 0
    function createDocument(parentId, name) { created++; return "book"; }
    function setOrientation(bookId, orientation) {}
    function setCoverPageNumber(bookId, page) { lastCover = [bookId, page]; }
}
"""})
    (stage / "NativeEditor.qml").write_text("""import QtQuick
FocusScope {
    property bool ready: false
    property int openCount: 0
    property int closeCount: 0
    property int hideCount: 0
    property bool automatic: false
    property var openedCard: null
    signal notebookCreated(string bookId)
    signal sheetCreated(string cardId, string bookId, string pageId)
    signal returnToDecks
    signal error(string message)
    function warmUp() {}
    function prepareCard() { ready = false; }
    function openCard(card) {
        openCount++;
        openedCard = card;
        if (automatic) {
            ready = true;
            if (!card.page_id)
                sheetCreated(card.card_id, card.book_id || "11111111-1111-4111-8111-111111111111",
                    "22222222-2222-4222-8222-" + card.card_id.slice(-12));
        }
        return true;
    }
    property int saveCount: 0
    function savePage() { saveCount++; }
    function hideResponse() { hideCount++; ready = false; }
    function closeDocument() { closeCount++; ready = false; }
}
""")
    hook = (ROOT / "packaging/launcher-control.qmd").read_text()
    assert "REBUILD openDocument_helper\n            LOCATE BEFORE ALL" in hook
    assert "if (openAnkiNotebook(entry)) return;" in hook
    launch = "function openAnkiNotebook(entry) {" + hook.split(
        "function openAnkiNotebook(entry) {", 1)[1].split("\n            }", 1)[0] + "\n}"
    sidebar = "ArkControls.SidebarItem {" + hook.split(
        "id: openAnkiPure", 1)[1].split("\n        }\n    END TRAVERSE", 1)[0]
    sidebar = sidebar.replace('objectName: "openAnkiPure"', 'id: openAnkiPure; objectName: "openAnkiPure"')
    module("ark.controls", {"SidebarItem": """import QtQuick.Controls
Button { property string iconSource }
"""})
    (stage / "tst_sidebar.qml").write_text("""import QtQuick
import QtQuick.Layouts
import QtTest
import ark.controls as ArkControls
import net.asivery.AppLoad 1.0
Item {
    id: root
    width: 500; height: 500
    property bool hidden: false
    function hide() { hidden = true; }
    ColumnLayout { anchors.fill: parent; SIDEBAR_ITEM }
    TestCase {
        name: "AnkiSidebar"; when: windowShown
        function test_opens_anki_directly_and_hides_sidebar() {
            AppLoadLauncher.launches = [];
            var button = findChild(root, "openAnkiPure");
            compare(button.text, "Anki");
            mouseClick(button);
            verify(root.hidden);
            compare(AppLoadLauncher.launches, ["ankimarkable-pure"]);
        }
    }
}
""".replace("SIDEBAR_ITEM", sidebar))
    (stage / "tst_notebook_launcher.qml").write_text("""import QtQuick
import QtTest
import net.asivery.AppLoad 1.0
Item {
    id: host
    property string notebook: "anki-notebook"
    function ankiNotebookId() { return notebook; }
    QtObject {
        id: ankiPureNotebookLibrary
        property var applications: [{id:"ankimarkable-pure"}]
    }
    LAUNCH_FUNCTION
    TestCase {
        name: "NotebookLauncher"; when: windowShown
        function init() {
            host.notebook = "anki-notebook";
            AppLoadLauncher.launches = [];
            ankiPureNotebookLibrary.applications = [{id:"ankimarkable-pure"}];
        }
        function test_anki_notebook_opens_the_app() {
            verify(host.openAnkiNotebook({id:"anki-notebook"}));
            compare(AppLoadLauncher.launches, ["ankimarkable-pure"]);
        }
        function test_other_notebooks_keep_the_native_editor() {
            verify(!host.openAnkiNotebook({id:"personal-notebook"}));
            verify(!host.openAnkiNotebook(null));
            compare(AppLoadLauncher.launches.length, 0);
        }
        function test_missing_app_keeps_notebook_accessible() {
            ankiPureNotebookLibrary.applications = [];
            verify(!host.openAnkiNotebook({id:"anki-notebook"}));
            compare(AppLoadLauncher.launches.length, 0);
        }
        function test_no_configured_notebook_cannot_launch() {
            host.notebook = "";
            verify(!host.openAnkiNotebook({id:"anki-notebook"}));
            compare(AppLoadLauncher.launches.length, 0);
        }
    }
}
""".replace("LAUNCH_FUNCTION", launch))

    (stage / "tst_review.qml").write_text("""import QtQuick
import QtTest
Rectangle {
    width: 1404; height: 1872
    Main { id: app }
    TestCase {
        name: "ReviewViews"; when: windowShown
        function front() {
            return {kind:"card", card_id:"1", deck_name:"Test", revealed:false,
                    question_image:"", answer_image:"", intervals:["1m","6m","10m","4d"]};
        }
        function item(name) { return findChild(app, name); }
        function init() {
            app.applyState({kind:"home",decks:[]});
            item("writingArea").openCount = 0;
            item("writingArea").closeCount = 0;
            item("writingArea").hideCount = 0;
            item("writingArea").saveCount = 0;
        }
        function open() {
            app.applyState(front());
            tryCompare(item("writingArea"), "openCount", 1);
        }
        function reveal() {
            var back = front(); back.revealed = true; app.applyState(back);
        }
        function test_reveal_keeps_writing_ready_above_ratings_data() {
            return [{tag:"Compact",fraction:0.28},{tag:"Balanced",fraction:0.45},{tag:"Large",fraction:0.65}];
        }
        function test_reveal_keeps_writing_ready_above_ratings(data) {
            app.writingFraction = data.fraction;
            open();
            var height = item("cardBox").height;
            verify(item("writingArea").visible);
            verify(item("writingLoading").visible);
            verify(item("writingSpinner").running);
            item("writingArea").ready = true;
            verify(!item("writingLoading").visible);
            verify(item("revealButton").visible);
            compare(item("revealButton").background.color, "#000000");
            compare(item("revealButton").contentItem.color, "#ffffff");
            compare(item("revealButton").text, "Show answer →");
            verify(!item("ratings").visible);
            reveal();
            compare(app.writingFraction, 0.45);
            compare(item("writingArea").height, app.height * 0.45);
            compare(item("writingLabel").text, "YOUR HANDWRITTEN RESPONSE");
            fuzzyCompare(item("cardBox").height, height - item("ratings").height + app.height * (data.fraction - 0.45), 0.01);
            verify(item("questionImage").visible);
            verify(item("answerImage").visible);
            compare(item("questionImage").parent, item("answerImage").parent);
            verify(item("writingArea").visible);
            verify(item("writingArea").ready);
            compare(item("ratings").y + item("ratings").height, item("writingLabel").y);
            compare(item("writingArea").y + item("writingArea").height, app.height);
            verify(!item("writingLoading").visible);
            verify(!item("revealButton").visible);
            verify(item("ratings").visible);
            compare(item("writingArea").closeCount, 0);
            compare(item("writingArea").hideCount, 0);
            compare(item("writingArea").saveCount, 1);
            compare(item("writingArea").openCount, 1);
        }
        function test_rating_works_while_native_page_stays_ready() {
            open(); item("writingArea").ready = true; reveal();
            verify(item("writingArea").ready);
            mouseClick(item("ratings"), 875, 56);
            compare(item("bridge").lastRequest.action, "answer");
            compare(item("bridge").lastRequest.grade, "good");
        }
        function test_same_card_again_reopens_writing() {
            open(); reveal(); app.applyState(front());
            tryCompare(item("writingArea"), "openCount", 2);
            verify(item("writingArea").visible);
            verify(!item("answerImage").visible);
        }
        function test_sheet_acknowledgment_does_not_reopen_page() {
            open();
            var state = front(); state.book_id = "book"; state.page_id = "page";
            app.applyState(state);
            compare(item("writingArea").openCount, 1);
        }
    }
}
""")
    (stage / "tst_writing.qml").write_text("""import QtQuick
import QtTest
import xofm.libs.library
Rectangle {
    width: 1404; height: 1872
    id: screen
    Item {
        id: appWindow; width: 1404; height: 1872; z: 100
        ProductionNativeEditor { id: writing; width: 1404; height: 500; y: 1372 }
    }
    Component { id: layerComponent; Item {} }
    QtObject {
        id: toolbar
        property bool visible: true
        property int shownToolsCount: 8
        property bool expanded: true
        property int position: 2
        property int preference: 2
        property int innerHeight: 100
    }
    Component {
        id: editorComponent
        FocusScope {
            property string currentPageId: "old"
            property int currentPage: 3
            property bool isLoading: true
            property var document: ({id:"book"})
            property alias scene: scene
            property alias surface: surface
            function enterTextMode(focus) {}
            function exitTextMode() {}
            function close() { document = null; }
            function openDocumentOnPage(book, index) { document = book; openPage(index); }
            function openPage(index) {
                isLoading = true;
                currentPage = index;
                currentPageId = document.idForPage(index);
                Qt.callLater(() => isLoading = false);
            }
            FocusScope {
                id: scene; focus: true
                function resetViewport() {}
                property var controller: QtObject {
                    property int clearCount: 0
                    property int ink: 0
                    property int flushCount: 0
                    function clearLines() { clearCount++; ink = 0; }
                    function clearRootDocument() { rootDocumentLength = 0; text = ""; }
                    property int rootDocumentLength: 0
                    property string text: ""
                    function replaceText(value) { text = value; rootDocumentLength = value.length; }
                    function requestDeferredCompleted() { flushCount++; }
                }
                property var viewport: ({})
                property var tileManager: ({})
            }
            QtObject {
                id: surface
                property int transforms: 0
                function updateTransform() { transforms++; }
            }
        }
    }
    TestCase {
        name: "WritingReadiness"; when: windowShown
        function init() {
            writing.prepareCard();
            LibraryController.lastCover = null;
            if (writing.editor) writing.editor.destroy();
            if (writing.surfaceLayer) writing.surfaceLayer.destroy();
            writing.surfaceLayer = layerComponent.createObject(screen, {width:1404, height:1872, z:101});
            writing.editor = editorComponent.createObject(writing);
            writing.drawingScene = writing.editor.scene;
            writing.inputSurface = writing.editor.surface;
            writing.writingToolbar = toolbar;
            writing.toolbarPositioner = toolbar;
            toolbar.visible = true;
            writing.expectedPageId = "wanted";
        }
        function test_wrong_or_loading_page_never_becomes_ready() {
            writing.editor.isLoading = false;
            verify(!writing.ready);
            writing.editor.isLoading = true;
            writing.editor.currentPageId = "wanted";
            verify(!writing.ready);
            writing.editor.isLoading = false;
            tryCompare(writing, "ready", true);
            verify(writing.drawingScene.activeFocus);
            verify(writing.inputSurface.transforms > 0);
            compare(writing.editor.parent, writing.surfaceLayer);
            compare(writing.surfaceLayer.parent, appWindow.parent);
            verify(writing.surfaceLayer.z > appWindow.z);
            compare(writing.editor.mapToItem(screen, 0, 0), writing.mapToItem(screen, 0, 0));
        }
        function test_hidden_toolbar_never_invites_writing() {
            toolbar.visible = false;
            writing.editor.currentPageId = "wanted";
            writing.editor.isLoading = false;
            verify(!writing.ready);
            toolbar.visible = true;
            tryCompare(writing, "ready", true);
        }
        function test_new_notebook_cover_and_multiple_decks_reuse_pages() {
            LibraryController.created = 0;
            Library.book.pages = ["cover"];
            writing.editor.document = null;
            writing.openCard({card_id:"one", deck_name:"First"});
            tryCompare(writing, "ready", true);
            compare(LibraryController.created, 1);
            compare(Library.book.pageCount, 2);
            compare(writing.editor.currentPageId, "page-1");
            compare(writing.drawingScene.controller.clearCount, 1);
            compare(LibraryController.lastCover, ["book",0]);
            writing.closeDocument();
            writing.openCard({card_id:"two", deck_name:"Second", book_id:"book", page_id:"page-1"});
            tryCompare(writing, "ready", true);
            compare(Library.book.pageCount, 2);
            compare(writing.editor.currentPageId, "page-1");
            writing.closeDocument();
            writing.openCard({card_id:"one", deck_name:"First", book_id:"book", page_id:"page-1"});
            tryCompare(writing, "ready", true);
            compare(Library.book.pageCount, 2);
            compare(LibraryController.created, 1);
            compare(writing.editor.currentPageId, "page-1");
        }
        function test_new_attempt_clears_ink_and_text_once_on_the_shared_page() {
            Library.book.pages = ["cover", "scratch"];
            writing.editor.document = Library.book;
            writing.openCard({card_id:"one", book_id:"book", page_id:"scratch"});
            tryCompare(writing, "ready", true);
            const controller = writing.drawingScene.controller;
            controller.ink = 5;
            controller.replaceText("old answer");
            writing.hideResponse();
            compare(controller.ink, 5);
            compare(writing.editor.document, Library.book);
            writing.prepareCard();
            writing.openCard({card_id:"two", book_id:"book", page_id:"scratch"});
            tryCompare(writing, "ready", true);
            compare(controller.ink, 0);
            compare(controller.text, "");
            compare(controller.clearCount, 2);
            controller.ink = 7;
            writing.activateInput();
            compare(controller.ink, 7);
            writing.hideResponse();
            writing.prepareCard();
            writing.openCard({card_id:"two", book_id:"book", page_id:"scratch"});
            tryCompare(writing, "ready", true);
            compare(controller.ink, 0);
            compare(controller.clearCount, 3);
            compare(Library.book.pageCount, 2);
        }
        function test_portal_tracks_geometry_and_visibility() {
            writing.editor.currentPageId = "wanted";
            writing.editor.isLoading = false;
            tryCompare(writing, "ready", true);
            writing.y = 1200;
            writing.refreshSurface();
            compare(writing.editor.mapToItem(screen, 0, 0), writing.mapToItem(screen, 0, 0));
            writing.visible = false;
            verify(!writing.surfaceLayer.visible);
            writing.visible = true;
            verify(writing.surfaceLayer.visible);
            writing.y = 1372;
        }
        function test_ordinary_edit_work_does_not_disable_writing() {
            writing.editor.currentPageId = "wanted";
            writing.editor.isLoading = false;
            tryCompare(writing, "ready", true);
            writing.editor.isLoading = true;
            verify(writing.ready);
            writing.prepareCard();
            verify(!writing.ready);
            compare(writing.editor.parent, writing.surfaceLayer);
            verify(!writing.surfaceLayer.visible);
        }
        function test_cover_is_written_once_and_remains_the_cover_after_writing() {
            var controller = writing.drawingScene.controller;
            writing.writeCover();
            verify(controller.text.indexOf("ANKI") === 0);
            compare(LibraryController.lastCover, ["book", 0]);
            compare(controller.flushCount, 1);
            writing.writeCover();
            compare(controller.flushCount, 1);
            writing.closeDocument();
            compare(LibraryController.lastCover, ["book", 0]);
            compare(controller.flushCount, 2);
        }
        function test_browsing_blank_page_keeps_cover() {
            writing.closeDocument();
            compare(LibraryController.lastCover, null);
            compare(writing.drawingScene.controller.flushCount, 1);
        }
        function test_close_clears_readiness_and_expected_page() {
            writing.editor.currentPageId = "wanted";
            writing.editor.isLoading = false;
            tryCompare(writing, "ready", true);
            writing.closeDocument();
            verify(!writing.ready);
            compare(writing.expectedPageId, "");
            compare(writing.editor.document, null);
            verify(!writing.surfaceLayer.visible);
        }
    }
}
""")

    (stage / "fonts.conf").write_text(
        f"<fontconfig><dir>{ROOT / 'assets/fonts'}</dir>"
        f"<cachedir>{stage / 'font-cache'}</cachedir></fontconfig>"
    )
    if endpoint:
        shutil.copy(ROOT / "tests/tst_workflow.qml", stage / "tst_workflow.qml")
        (stage / "Workflow.js").write_text(f'var endpoint = "{endpoint}";\n')
    environment = os.environ | {
        "QT_QPA_PLATFORM": "offscreen",
        "QT_QPA_PLATFORM_PLUGIN_PATH": str(TARGET / "usr/lib/plugins/platforms"),
        "QML_IMPORT_PATH": str(TARGET / "usr/lib/qml"),
        "QT_QUICK_BACKEND": "software",
        "QT_QUICK_CONTROLS_STYLE": "Basic",
        "FONTCONFIG_FILE": str(stage / "fonts.conf"),
    }
    subprocess.run([
        str(TARGET / "lib/ld-linux-aarch64.so.1"), "--library-path",
        f"{TARGET}/usr/lib:{TARGET}/lib", str(HOST / "usr/bin/qmltestrunner"),
        "-import", str(stage / "imports"), "-input", str(stage),
    ], env=environment, check=True)
