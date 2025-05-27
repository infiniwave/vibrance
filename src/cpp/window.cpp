#include "window.h"
#include <QApplication>
#include <QIcon>
#include "mainwindow.h"
#include <QtCore/QResource>
#include <QWidget>
#ifdef _WIN32
#include <windows.h>
#endif

static MainWindow* g_mainwindow = nullptr;

void show_widget_window(std::int32_t argc, std::int8_t** argv) {
    Q_INIT_RESOURCE(resources);
    QApplication app(argc, reinterpret_cast<char**>(argv));
    MainWindow window;
    g_mainwindow = &window;
    window.setWindowIcon(QIcon(":/app.ico"));
    window.setWindowTitle("Vibrance");
    window.resize(800, 600); // match the UI default size
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

void mediaplayer_set_track(std::uintptr_t mediaplayer, rust::String title, rust::String artists, rust::String album, double duration) {
    if (mediaplayer) {
        reinterpret_cast<MediaPlayer*>(mediaplayer)->setTrack(std::string(title), std::string(artists), std::string(album), duration);
    }
}

void mediaplayer_set_paused(std::uintptr_t mediaplayer, bool paused) {
    if (mediaplayer) {
        reinterpret_cast<MediaPlayer*>(mediaplayer)->setPaused(paused);
    }
}


void* get_mainwindow_hwnd() {
#ifdef _WIN32
    if (g_mainwindow != nullptr) {
        return reinterpret_cast<void*>(g_mainwindow->winId());
    }
#endif
    return nullptr;
}
