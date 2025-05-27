#include "window.h"
#include <QApplication>
#include "mainwindow.h"

static MainWindow* g_mainwindow = nullptr;

void show_widget_window(std::int32_t argc, std::int8_t** argv) {
    QApplication app(argc, reinterpret_cast<char**>(argv));
    MainWindow window;
    g_mainwindow = &window;
    window.setWindowTitle("Qt MediaPlayer Widget from Rust");
    window.resize(1021, 150); // match the UI default size
    window.show();
    app.exec();
    g_mainwindow = nullptr;
}

std::uintptr_t get_mainwindow_mediaplayer() {
    if (g_mainwindow) {
        return reinterpret_cast<std::uintptr_t>(g_mainwindow->getMediaPlayer());
    }
    return 0;
}

void mediaplayer_set_progress(std::uintptr_t mediaplayer, double value) {
    if (mediaplayer) {
        reinterpret_cast<MediaPlayer*>(mediaplayer)->setProgress(value);
    }
}
