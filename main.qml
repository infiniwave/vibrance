import QtQuick 2.15
import QtQuick.Controls 2.15
import MusicPlayer 1.0
import Qt.labs.platform 1.1

ApplicationWindow {
    visible: true
    width: 600
    height: 500
    title: qsTr("Vibrance")
    Connections {
        target: music
        function onSync_progress(p: real): void {
            music.progress = p;
            console.log("Progress updated:", p);
        }
    }

    MusicPlayer { 
        id: music
        property real progress: 0.0
        property string file_path: ""
    }

    Column {
        anchors.centerIn: parent
        spacing: 20

        Text {
            text: "Welcome to Vibrance!"
            font.pointSize: 18
            horizontalAlignment: Text.AlignHCenter
        }

        Text {
            text: music.file_path.length > 0 ? music.file_path : "No file selected"
            font.pointSize: 14
            color: "#888"
            horizontalAlignment: Text.AlignHCenter
        }
        Button {
            text: "Choose File"
            onClicked: fileDialog.open()
        }
        Button {
            text: "Play"
            onClicked: music.play()
        }
        Button {
            text: "Pause"
            onClicked: music.pause()
        }
        Slider {
            id: progressSlider
            from: 0
            to: 1
            value: music.progress
            stepSize: 0.001
            width: 300
            enabled: music.file_path.length > 0
            onMoved: music.seek(value)
        }
        Slider {
            id: volumeSlider
            from: 0
            to: 1
            value: 1
            stepSize: 0.01
            width: 300
            onMoved: music.set_volume(value)
            enabled: muteCheckBox.checked === false
        }
        CheckBox {
            id: muteCheckBox
            text: "Mute"
            checked: music.muted
            onCheckedChanged: music.set_muted(checked)
        }
    }

    FileDialog {
        id: fileDialog
        title: "Select a music file"
        nameFilters: ["Audio files (*.mp3 *.wav *.ogg)", "All files (*)"]
        onAccepted: {
            music.set_file(fileDialog.file.toString().replace('file:///', ''))
            music.file_path = fileDialog.file.toString().replace('file:///', '');
        }
    }
}
