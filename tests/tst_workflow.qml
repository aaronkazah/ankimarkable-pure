import QtQuick
import QtTest
import "Workflow.js" as Workflow

Rectangle {
    id: screen
    width: 1404
    height: 1872
    Component { id: application; Main {} }

    TestCase {
        name: "RealReviewWorkflow"
        when: windowShown
        property var app

        function server(request) {
            var xhr = new XMLHttpRequest();
            xhr.open("POST", Workflow.endpoint, false);
            xhr.send(JSON.stringify(request));
            compare(xhr.status, 200, xhr.responseText);
            return JSON.parse(xhr.responseText);
        }
        function item(name) { return findChild(app, name); }
        function start(scenario) {
            server({action:"_reset", scenario:scenario});
            app = createTemporaryObject(application, screen, {busy:false});
            verify(app !== null);
            item("bridge").live = true;
            item("writingArea").automatic = true;
            app.send("home");
            tryCompare(app, "busy", false);
            compare(app.model.kind, "home");
        }
        function writingReady() {
            tryVerify(() => app.model.kind === "card" && !app.model.revealed && !!app.model.page_id);
            tryCompare(item("writingArea"), "ready", true);
            compare(item("writingArea").openedCard.card_id, app.model.card_id);
            verify(!item("writingLoading").visible);
            tryCompare(item("questionImage"), "status", Image.Ready);
            verify(item("questionImage").height > 0);
        }
        function reveal() {
            mouseClick(item("revealButton"));
            tryVerify(() => app.model.revealed === true && !app.busy);
            verify(item("questionImage").visible);
            verify(item("answerImage").visible);
            tryCompare(item("answerImage"), "status", Image.Ready);
            verify(!item("writingArea").visible);
            verify(item("ratings").visible);
        }
        function grade(index) {
            mouseClick(item("ratings"), (index + 0.5) * item("ratings").width / 4, 56);
            tryCompare(app, "busy", false);
        }
        function cleanup() {
            if (app) app.unloading();
            app = null;
        }
        function test_all_ratings_advance_and_persist_data() {
            return [
                {tag:"Again", index:0}, {tag:"Hard", index:1},
                {tag:"Good", index:2}, {tag:"Easy", index:3}
            ];
        }
        function test_all_ratings_advance_and_persist(data) {
            start("multiple");
            mouseClick(item("reviewDeckButton"));
            writingReady();
            var first = app.model.card_id;
            var firstImage = app.model.question_image;
            var page = app.model.page_id;
            var button = item("revealButton");
            compare(button.text, "Show answer →");
            compare(button.background.color, "#000000");
            compare(button.contentItem.color, "#ffffff");
            waitForRendering(button);
            var pixels = grabImage(app);
            var point = button.mapToItem(app, 10, 10);
            compare(pixels.red(point.x, point.y), 0);
            compare(pixels.green(point.x, point.y), 0);
            compare(pixels.blue(point.x, point.y), 0);
            reveal();
            grade(data.index);
            writingReady();
            verify(app.model.card_id !== first);
            verify(app.model.question_image !== firstImage);
            compare(item("reviewNotice").text, "Saved · Next card");
            var database = server({action:"_inspect"});
            compare(database.reviews, [[first, data.index + 1]]);
            compare(database.notes.book_id, "11111111-1111-4111-8111-111111111111");
            compare(database.notes.page_id, page);
            app.unloading();
            server({action:"_restart"});
            app.send("home");
            tryCompare(app, "busy", false);
            mouseClick(item("reviewDeckButton"));
            writingReady();
            verify(app.model.card_id !== first);
            compare(server({action:"_inspect"}).reviews, database.reviews);
        }
        function test_double_rating_click_records_one_review_and_undo_restores_card() {
            start("multiple");
            mouseClick(item("reviewDeckButton"));
            writingReady();
            var first = app.model.card_id;
            reveal();
            mouseClick(item("ratings"), 875, 56);
            mouseClick(item("ratings"), 875, 56);
            writingReady();
            compare(server({action:"_inspect"}).reviews, [[first, 3]]);
            app.send("undo");
            tryCompare(app, "busy", false);
            writingReady();
            compare(app.model.card_id, first);
            compare(server({action:"_inspect"}).reviews.length, 0);
        }
        function test_one_card_learning_repeat_is_explained_and_easy_finishes() {
            start("single");
            mouseClick(item("reviewDeckButton"));
            writingReady();
            var first = app.model.card_id;
            var page = app.model.page_id;
            for (var index = 0; index < 3; index++) {
                reveal();
                grade(index);
                writingReady();
                compare(app.model.card_id, first);
                compare(app.model.page_id, page);
                compare(item("reviewNotice").text, "Saved · Same card: another learning step");
            }
            reveal();
            grade(3);
            compare(app.model.kind, "finished");
            var database = server({action:"_inspect"});
            compare(database.reviews, [[first,1], [first,2], [first,3], [first,4]]);
            compare(database.notes.book_id, "11111111-1111-4111-8111-111111111111");
            compare(database.notes.page_id, page);
        }
        function test_preview_navigation_and_reopen_reuse_scratch_page_without_grading() {
            start("future");
            var before = server({action:"_inspect"}).cards;
            mouseClick(item("previewDeckButton"));
            writingReady();
            var first = app.model.card_id;
            var page = app.model.page_id;
            reveal();
            mouseClick(item("nextCardButton"));
            writingReady();
            verify(app.model.card_id !== first);
            reveal();
            mouseClick(item("previousCardButton"));
            writingReady();
            compare(app.model.card_id, first);
            compare(app.model.page_id, page);
            mouseClick(item("homeButton"));
            tryCompare(app, "busy", false);
            server({action:"_restart"});
            mouseClick(item("previewDeckButton"));
            writingReady();
            compare(app.model.page_id, page);
            var database = server({action:"_inspect"});
            compare(database.reviews.length, 0);
            compare(database.cards, before);
            compare(database.notes.page_id, page);
        }
    }
}
