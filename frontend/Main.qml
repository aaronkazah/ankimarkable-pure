import QtCore as QtCore
import QtQuick
import QtQuick.Controls
import QtQuick.Window
import net.asivery.AppLoad 1.0
import xofm.libs.epaper as Epaper

Rectangle {
    id: root
    anchors.fill: parent
    color: "white"
    signal close
    property var model: ({
            kind: "loading"
        })
    property bool busy: true
    property bool pendingHome: false
    property bool requestTimedOut: false
    property int requestTimeoutMs: 30000
    property var expandedDecks: ({})
    readonly property var visibleDecks: {
        const result = [];
        let hiddenBelow = -1;
        for (const deck of model.decks || []) {
            if (hiddenBelow >= 0 && deck.level > hiddenBelow) continue;
            hiddenBelow = -1;
            result.push(deck);
            if (deck.has_children && !isExpanded(deck)) hiddenBelow = deck.level;
        }
        return result;
    }

    function isExpanded(deck) {
        return expandedDecks[deck.id] === undefined ? deck.level === 0 : expandedDecks[deck.id];
    }

    function toggleDeck(deck) {
        expandedDecks = Object.assign({}, expandedDecks, {[deck.id]: !isExpanded(deck)});
    }
    property alias writingFraction: preferences.writingFraction
    QtCore.Settings {
        id: preferences
        location: "file:///home/root/.ankimarkable-pure/display.ini"
        property real writingFraction: 0.45
    }
    property var pendingNativeCard: null
    property string questionImageSource: ""
    property string answerImageSource: ""
    property string ratedCardId: ""
    property string reviewNotice: ""
    readonly property var ratings: ["Again", "Hard", "Good", "Easy"]
    readonly property var shades: ["#444444", "#888888", "#bcbcbc", "#eeeeee"]

    component PaperButton: Button {
        property bool primary: false
        focusPolicy: Qt.NoFocus
        background: Rectangle {
            color: parent.primary ? "black" : "white"
            border.color: "black"
            border.width: parent.flat ? 0 : 2
        }
        contentItem: Text {
            text: parent.text
            font: parent.font
            color: parent.primary ? "white" : "black"
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
    }

    function send(action, fields) {
        if (action !== "record_sheet" && action !== "record_notebook") {
            if (busy) {
                if (action === "home") pendingHome = true;
                return;
            }
            busy = true;
            requestTimedOut = false;
            requestTimer.restart();
            if (action === "answer") ratedCardId = model.card_id;
        }
        bridge.sendMessage(1, JSON.stringify(Object.assign({
            action: action
        }, fields || {})));
    }

    function applyState(next) {
        requestTimer.stop();
        requestTimedOut = false;
        if (pendingHome) {
            pendingHome = false;
            pendingNativeCard = null;
            busy = false;
            send("home");
            return;
        }
        if (JSON.stringify(model) === JSON.stringify(next)) {
            busy = false;
            return;
        }
        if (ratedCardId && next.kind === "card" && !next.revealed && model.revealed) {
            reviewNotice = next.card_id === ratedCardId
                ? "Saved · Same card: another learning step"
                : "Saved · Next card";
            ratedCardId = "";
        } else if (next.kind !== "card" || next.revealed) {
            reviewNotice = "";
            if (next.kind !== "card") ratedCardId = "";
        }
        if (next.kind === "card" && !next.revealed && (next.card_id !== model.card_id || model.kind !== "card" || model.revealed)) {
            nativeEditor.prepareCard();
            pendingNativeCard = next;
        } else if (next.kind !== "card" || next.revealed) {
            pendingNativeCard = null;
            nativeEditor.hideResponse();
        }
        if (next.card_id !== model.card_id || next.revealed !== model.revealed)
            cardScroll.contentY = 0;
        model = next;
        if (next.kind === "card") {
            questionImageSource = next.question_image || "";
            answerImageSource = next.answer_image || "";
        }
        busy = false;
        if (next.kind === "home") Qt.callLater(nativeEditor.warmUp);

    }

    Timer {
        id: requestTimer
        interval: root.requestTimeoutMs
        onTriggered: root.requestTimedOut = true
    }

    function unloading() {
        pendingNativeCard = null;
        nativeEditor.closeDocument();
    }

    Connections {
        target: root.Window.window
        enabled: root.pendingNativeCard !== null
        function onFrameSwapped() {
            const card = root.pendingNativeCard;
            root.pendingNativeCard = null;
            nativeEditor.openCard(card);
        }
    }

    Epaper.ScreenModeItem {
        width: root.width
        height: root.model.kind === "card" ? header.height : root.height
        mode: Epaper.ScreenModeItem.UI
        z: 2
    }

    Epaper.ScreenModeItem {
        y: notesLabel.y
        width: root.width
        height: notesLabel.height
        visible: root.model.kind === "card" && !root.model.revealed
        mode: Epaper.ScreenModeItem.UI
        z: 2
    }

    Epaper.ScreenModeItem {
        y: ratingBar.y
        width: root.width
        height: ratingBar.height
        visible: root.model.kind === "card" && root.model.revealed
        mode: Epaper.ScreenModeItem.UI
        z: 2
    }

    AppLoad {
        id: bridge
        objectName: "bridge"
        applicationID: "ankimarkable-pure"
        onMessageReceived: (type, contents) => {
            if (type === 101)
                root.applyState(JSON.parse(contents));
        }
    }

    Rectangle {
        id: header
        height: 88
        width: parent.width
        color: "white"
        border.color: "#dddddd"
        border.width: 1
        PaperButton {
            id: homeButton
            objectName: "homeButton"
            x: 16
            height: 80
            font.pixelSize: 32
            anchors.verticalCenter: parent.verticalCenter
            text: root.model.kind === "card" ? "Decks" : "Anki"
            flat: true
            onClicked: root.send("home")
        }
        Column {
            anchors.left: homeButton.right
            anchors.right: settingsButton.left
            anchors.margins: 20
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
            Text {
                objectName: "deckTitle"
                width: parent.width
                font.pixelSize: 34
                font.bold: true
                text: (root.model.deck_name || "Anki").split("::").pop()
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignHCenter
            }
            Text {
                objectName: "deckSubtitle"
                width: parent.width
                visible: root.model.kind === "card"
                font.pixelSize: 24
                text: {
                    const path = (root.model.deck_name || "").split("::");
                    const context = path.length > 1 ? path.slice(0, -1).join(" / ") + " · " : "";
                    return context + (root.model.preview
                        ? "Preview " + root.model.preview.number + " of " + root.model.preview.total
                        : "Scheduled review");
                }
                elide: Text.ElideRight
                horizontalAlignment: Text.AlignHCenter
            }
        }
        PaperButton {
            id: settingsButton
            objectName: "settingsButton"
            anchors.right: menuButton.visible ? menuButton.left : closeButton.left
            anchors.verticalCenter: parent.verticalCenter
            text: "Settings"
            width: 180
            height: 80
            font.pixelSize: 28
            flat: true
            onClicked: settingsMenu.open()
        }
        Menu {
            id: settingsMenu
            objectName: "settingsMenu"
            font.pixelSize: 30
            x: Math.max(0, settingsButton.x + settingsButton.width - width)
            y: header.height
            MenuItem {
                text: "Writing area size"
                font.bold: true
                enabled: false
            }
            MenuSeparator {}
            MenuItem {
                objectName: "writingSizeCompact"
                text: "Compact"
                checkable: true
                checked: root.writingFraction === 0.28
                onTriggered: root.writingFraction = 0.28
            }
            MenuItem {
                objectName: "writingSizeBalanced"
                text: "Balanced"
                checkable: true
                checked: root.writingFraction === 0.45
                onTriggered: root.writingFraction = 0.45
            }
            MenuItem {
                objectName: "writingSizeLarge"
                text: "Large"
                checkable: true
                checked: root.writingFraction === 0.65
                onTriggered: root.writingFraction = 0.65
            }
        }
        PaperButton {
            id: menuButton
            objectName: "cardMenuButton"
            visible: root.model.kind === "card" && !root.model.preview
            enabled: !root.busy
            anchors.right: closeButton.left
            anchors.verticalCenter: parent.verticalCenter
            text: "Card actions"
            width: 180
            height: 80
            font.pixelSize: 27
            flat: true
            onClicked: cardMenu.open()
        }
        PaperButton {
            id: closeButton
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            text: "×"
            width: 80
            height: 80
            font.pixelSize: 36
            flat: true
            onClicked: root.close()
        }
        Menu {
            id: cardMenu
            font.pixelSize: 32
            x: parent.width - width - 12
            y: header.height
            MenuItem {
                text: "Undo last review"
                onTriggered: root.send("undo")
            }
            MenuItem {
                text: "Flag card"
                visible: root.model.kind === "card" && !root.model.preview
                onTriggered: flagMenu.open()
            }
            MenuItem {
                text: "Hide until tomorrow"
                visible: root.model.kind === "card" && !root.model.preview
                onTriggered: root.send("bury")
            }
            MenuItem {
                text: "Pause this note"
                visible: root.model.kind === "card" && !root.model.preview
                onTriggered: root.send("suspend")
            }
        }
        Menu {
            id: flagMenu
            font.pixelSize: 32
            x: parent.width - width - 12
            y: header.height
            Repeater {
                model: ["Remove flag", "Red", "Orange", "Green", "Blue", "Pink", "Turquoise", "Purple"]
                MenuItem {
                    required property int index
                    required property string modelData
                    text: modelData
                    onTriggered: root.send("flag", {
                        flag: index
                    })
                }
            }
        }
    }

    ListView {
        id: deckList
        objectName: "deckList"
        anchors.top: header.bottom
        anchors.bottom: parent.bottom
        width: parent.width
        visible: root.model.kind === "home"
        model: root.visibleDecks
        clip: true
        pressDelay: 180
        boundsBehavior: Flickable.StopAtBounds
        header: Label {
            width: root.width
            height: 80
            padding: 24
            text: "Expand a group · Review due cards or preview without changing progress"
            font.pixelSize: 26
        }
        delegate: Item {
            required property var modelData
            objectName: "deckRow-" + modelData.id
            width: root.width
            height: 148
            PaperButton {
                id: expandButton
                objectName: "toggleDeck-" + modelData.id
                x: 24 + modelData.level * 36
                anchors.verticalCenter: parent.verticalCenter
                width: 64
                height: 80
                visible: modelData.has_children === true
                text: root.isExpanded(modelData) ? "−" : "+"
                font.pixelSize: 36
                flat: true
                onClicked: root.toggleDeck(modelData)
            }
            Column {
                x: expandButton.x + expandButton.width + 12
                anchors.verticalCenter: parent.verticalCenter
                width: reviewButton.x - x - 24
                spacing: 12
                Label {
                    width: parent.width
                    text: modelData.name
                    wrapMode: Text.Wrap
                    font.pixelSize: 32
                }
                Label {
                    width: parent.width
                    text: modelData.total + " cards · " + modelData.new + " new · " + modelData.learn + " learning · " + modelData.due + " due"
                    elide: Text.ElideRight
                    font.pixelSize: 25
                    color: "#555555"
                }
            }
            PaperButton {
                id: reviewButton
                objectName: "reviewDeckButton"
                anchors.right: previewButton.left
                anchors.rightMargin: 16
                anchors.verticalCenter: parent.verticalCenter
                width: 250
                height: 64
                text: "Review due →"
                primary: true
                font.pixelSize: 27
                enabled: !root.busy
                onClicked: root.send("review", {deck_id: modelData.id})
            }
            PaperButton {
                id: previewButton
                objectName: "previewDeckButton"
                anchors.right: parent.right
                anchors.rightMargin: 32
                anchors.verticalCenter: parent.verticalCenter
                text: "Preview cards"
                width: 230
                height: 64
                font.pixelSize: 26
                enabled: !root.busy && modelData.total > 0
                onClicked: root.send("preview", {deck_id: modelData.id})
            }
        }
        ScrollBar.vertical: ScrollBar {}
    }

    Rectangle {
        id: cardPanel
        objectName: "cardBox"
        x: 0
        y: header.height
        width: root.width
        height: root.model.revealed ? root.height - header.height - ratingBar.height : notesLabel.y - header.height
        visible: root.model.kind === "card"
        color: "white"
        border.color: "#dddddd"
        border.width: 1
        Epaper.ScreenModeItem {
            anchors.fill: parent
            mode: Epaper.ScreenModeItem.Animation
            z: 1
        }
        Label {
            objectName: "reviewNotice"
            x: 24
            y: 14
            font.pixelSize: 24
            text: root.reviewNotice
        }
        Flickable {
            id: cardScroll
            objectName: "cardScroll"
            x: 24
            y: root.reviewNotice ? 52 : 24
            width: parent.width - 48
            height: parent.height - y - (root.model.revealed ? 24 : 124)
            clip: true
            contentWidth: width
            contentHeight: cardContent.height
            Column {
                id: cardContent
                width: cardScroll.width
                spacing: 16
                Label {
                    text: "QUESTION"
                    font.pixelSize: 30
                    font.bold: true
                }
                Image {
                    id: questionImage
                    objectName: "questionImage"
                    width: parent.width
                    height: sourceSize.width > 0 ? width * sourceSize.height / sourceSize.width : 0
                    source: root.questionImageSource
                    cache: false
                    asynchronous: true
                    smooth: true
                }
                Rectangle {
                    width: parent.width
                    height: 2
                    color: "#dddddd"
                    visible: root.model.revealed === true
                }
                Label {
                    text: "ANSWER"
                    font.pixelSize: 30
                    font.bold: true
                    visible: root.model.revealed === true
                }
                Image {
                    id: answerImage
                    objectName: "answerImage"
                    width: parent.width
                    height: sourceSize.width > 0 ? width * sourceSize.height / sourceSize.width : 0
                    source: questionImage.status === Image.Ready ? root.answerImageSource : ""
                    visible: root.model.revealed === true
                    cache: false
                    asynchronous: true
                    smooth: true
                }
            }
            ScrollBar.vertical: ScrollBar {}
        }
        PaperButton {
            objectName: "revealButton"
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 24
            width: 330
            height: 76
            text: "Show answer →"
            primary: true
            font.pixelSize: 32
            visible: !root.model.revealed
            onClicked: root.send("reveal")
        }
        Label {
            anchors.centerIn: parent
            visible: questionImage.status === Image.Error || answerImage.status === Image.Error
            text: "Could not display this card. Reopen the deck."
            font.pixelSize: 30
        }
    }

    Label {
        id: notesLabel
        x: 24
        y: nativeEditor.y - height
        height: 44
        visible: root.model.kind === "card" && !root.model.revealed
        text: "YOUR HANDWRITTEN RESPONSE"
        font.pixelSize: 24
        verticalAlignment: Text.AlignVCenter
    }

    NativeEditor {
        id: nativeEditor
        objectName: "writingArea"
        x: 0
        y: root.height - height
        width: root.width
        height: root.height * (Qt.inputMethod.visible ? Math.max(0.60, root.writingFraction) : root.writingFraction)
        visible: root.model.kind === "card" && !root.model.revealed
        onNotebookCreated: bookId => root.send("record_notebook", {book_id: bookId})
        onSheetCreated: (cardId, bookId, pageId) => root.send("record_sheet", {
                card_id: cardId,
                book_id: bookId,
                page_id: pageId
            })
        onReturnToDecks: root.send("home")
        onError: message => root.applyState({
                kind: "message",
                title: "Editor",
                body: message
            })
    }

    Rectangle {
        objectName: "writingLoading"
        x: nativeEditor.x
        y: nativeEditor.y
        width: nativeEditor.width
        height: nativeEditor.height
        visible: nativeEditor.visible && !nativeEditor.ready
        color: "white"
        BusyIndicator {
            objectName: "writingSpinner"
            anchors.centerIn: parent
            width: 48
            height: 48
            running: parent.visible
            palette.dark: "black"
            palette.highlight: "black"
        }
        MouseArea {
            anchors.fill: parent
        }
    }

    Rectangle {
        id: ratingBar
        objectName: "ratings"
        anchors.bottom: parent.bottom
        width: parent.width
        height: 112
        visible: root.model.kind === "card" && root.model.revealed
        color: "white"
        Row {
            anchors.fill: parent
            visible: root.model.revealed === true && !root.model.preview
            Repeater {
                model: 4
                Rectangle {
                    required property int index
                    width: ratingBar.width / 4
                    height: ratingBar.height
                    color: root.shades[index]
                    border.color: "white"
                    border.width: 1
                    Text {
                        x: 0
                        y: 16
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        text: root.ratings[index]
                        color: index < 2 ? "white" : "black"
                        font.pixelSize: 32
                        font.bold: true
                    }
                    Text {
                        x: 0
                        y: 62
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        text: (root.model.intervals || [])[index] || ""
                        color: index < 2 ? "white" : "black"
                        font.pixelSize: 26
                    }
                    MouseArea {
                        anchors.fill: parent
                        enabled: !root.busy
                        onClicked: root.send("answer", {
                            grade: root.ratings[index].toLowerCase()
                        })
                    }
                }
            }
        }
        Row {
            anchors.fill: parent
            visible: root.model.revealed === true && !!root.model.preview
            PaperButton {
                objectName: "previousCardButton"
                width: ratingBar.width / 3
                height: ratingBar.height
                text: "Previous"
                font.pixelSize: 32
                enabled: root.model.preview?.number > 1
                onClicked: root.send("previous")
            }
            Label {
                width: ratingBar.width / 3
                height: ratingBar.height
                text: "Preview · no grading"
                font.pixelSize: 26
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
            }
            PaperButton {
                objectName: "nextCardButton"
                width: ratingBar.width / 3
                height: ratingBar.height
                text: "Next"
                font.pixelSize: 32
                enabled: root.model.preview?.number < root.model.preview?.total
                onClicked: root.send("next")
            }
        }
    }

    Rectangle {
        objectName: "requestStatus"
        anchors.top: header.bottom
        width: parent.width
        height: statusText.implicitHeight + 32
        color: "white"
        border.color: "#555555"
        visible: root.busy
        z: 10
        Label {
            id: statusText
            x: 24
            y: 16
            width: parent.width - 48
            font.pixelSize: 28
            wrapMode: Text.Wrap
            text: root.requestTimedOut
                ? "Anki is not responding. Close with × and reopen. If you just rated a card, check its progress before rating it again."
                : root.pendingHome ? "Returning to decks…" : "Loading…"
        }
    }

    Column {
        anchors.centerIn: parent
        width: parent.width - 80
        spacing: 20
        visible: ["loading", "message", "finished"].includes(root.model.kind)
        Label {
            width: parent.width
            text: root.model.title || (root.model.kind === "finished" ? "No cards due right now" : "Opening Anki…")
            font.pixelSize: 36
            font.bold: true
            horizontalAlignment: Text.AlignHCenter
        }
        Label {
            width: parent.width
            text: root.model.body || (root.model.kind === "finished" ? "You can preview all " + root.model.total + " cards without changing their schedule." : "")
            wrapMode: Text.Wrap
            font.pixelSize: 30
            horizontalAlignment: Text.AlignHCenter
        }
        PaperButton {
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.model.kind === "finished" && root.model.total > 0
            text: "Preview cards"
            font.pixelSize: 32
            onClicked: root.send("preview", {
                deck_id: root.model.deck_id
            })
        }
        PaperButton {
            anchors.horizontalCenter: parent.horizontalCenter
            visible: root.model.kind !== "loading"
            text: "Decks"
            font.pixelSize: 32
            onClicked: root.send("home")
        }
    }
}
