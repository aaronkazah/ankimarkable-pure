import QtQuick
import QtTest

Rectangle {
    id: screen
    width: 1404
    height: 1872
    Component { id: application; Main {} }
    TestCase {
        name: "Navigation"
        when: windowShown
        property var app
        function item(name) { return findChild(app, name); }
        function decks() {
            return [{id:"1",name:"Course",level:0,has_children:true,total:2,new:2,learn:0,due:0},
                    {id:"2",name:"Foundations",level:1,has_children:true,total:1,new:1,learn:0,due:0},
                    {id:"3",name:"Regression",level:2,has_children:false,total:1,new:1,learn:0,due:0},
                    {id:"4",name:"Math",level:1,has_children:false,total:1,new:1,learn:0,due:0}];
        }
        function init() {
            app = createTemporaryObject(application, screen, {busy:false});
            app.writingFraction = 0.45;
            app.applyState({kind:"home",decks:decks()});
        }
        function cleanup() { app.unloading(); app = null; }
        function test_both_card_images_decode_without_blocking_input() {
            verify(item("questionImage").asynchronous);
            verify(item("answerImage").asynchronous);
        }
        function test_returning_to_decks_retains_prepared_image_sources() {
            app.applyState({kind:"card",card_id:"1",revealed:false,question_image:"file:///question.png",answer_image:"file:///answer.png"});
            app.applyState({kind:"home",decks:decks()});
            compare(app.questionImageSource,"file:///question.png");
            compare(app.answerImageSource,"file:///answer.png");
        }
        function test_writing_area_sizes_preserve_the_current_card() {
            app.applyState({kind:"card",card_id:"1",revealed:false,preview:{number:1,total:1},question_image:"",answer_image:""});
            verify(item("writingSizeButton").visible);
            item("writingSizeLarge").triggered();
            compare(item("writingArea").height,app.height*0.65);
            compare(app.model.card_id,"1");
            item("writingSizeCompact").triggered();
            compare(item("writingArea").height,app.height*0.28);
            compare(item("bridge").lastRequest,null);
        }
        function test_no_cloud_sync_button() {
            compare(item("syncButton"), null);
            compare(item("cardMenuButton").text, "Card actions");
            verify(!item("cardMenuButton").visible);
        }
        function test_deck_groups_expand_and_collapse_without_opening_cards() {
            compare(item("deckList").count, 3);
            mouseClick(item("toggleDeck-2"));
            compare(item("deckList").count, 4);
            mouseClick(item("toggleDeck-1"));
            compare(item("deckList").count, 1);
            mouseClick(item("toggleDeck-1"));
            compare(item("deckList").count, 4);
            compare(item("bridge").lastRequest, null);
        }
        function test_tapping_a_deck_label_does_not_start_a_review() {
            mouseClick(item("deckRow-1"), 250, 65);
            compare(item("bridge").lastRequest, null);
        }
        function test_swiping_from_a_review_button_does_not_open_a_card() {
            var rows=[];
            for (var i=0;i<30;i++) rows.push({id:String(i),name:"Topic "+i,level:0,has_children:false,total:1,new:1,learn:0,due:0});
            app.applyState({kind:"home",decks:rows});
            var list=item("deckList");
            var button=item("reviewDeckButton");
            var point=button.mapToItem(list,button.width/2,button.height/2);
            mouseDrag(list, point.x, point.y, 0, -400, Qt.LeftButton, Qt.NoModifier, 100);
            verify(list.contentY>0);
            compare(item("bridge").lastRequest, null);
        }
        function test_decks_request_is_queued_while_a_card_opens() {
            app.send("preview",{deck_id:"3"});
            app.send("home");
            compare(item("bridge").lastRequest.action,"preview");
            app.applyState({kind:"card",card_id:"1",revealed:false,question_image:"",answer_image:""});
            compare(item("bridge").lastRequest.action,"home");
            verify(app.busy);
            compare(item("writingArea").openCount,0);
            app.applyState({kind:"home",decks:decks()});
            verify(!app.busy);
        }
        function test_request_timeout_is_visible_and_does_not_repeat_a_rating() {
            app.requestTimeoutMs=30;
            app.applyState({kind:"card",card_id:"1",revealed:true,question_image:"",answer_image:""});
            app.send("answer",{grade:"good"});
            tryCompare(app,"requestTimedOut",true);
            verify(item("requestStatus").visible);
            compare(item("bridge").lastRequest.action,"answer");
            app.send("answer",{grade:"good"});
            verify(app.busy);
            app.applyState({kind:"home",decks:decks()});
            verify(!app.requestTimedOut);
        }
    }
}
