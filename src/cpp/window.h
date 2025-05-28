#pragma once
#include <cstdint>
#include <cstddef>
#include <string>
#include "../../target/cxxbridge/rust/cxx.h"
#include "../../target/cxxbridge/vibrance/src/main.rs.h"

#ifdef __cplusplus
extern "C" {
#endif
void show_widget_window(std::int32_t argc, std::int8_t** argv);
std::uintptr_t get_mainwindow_mediaplayer();
std::uintptr_t get_mainwindow();
void mediaplayer_set_progress(std::uintptr_t mediaplayer, double value);
void mediaplayer_set_track(std::uintptr_t mediaplayer, rust::String title, rust::String artists, rust::String album, double duration);
void mediaplayer_set_paused(std::uintptr_t mediaplayer, bool paused);
void* get_mainwindow_hwnd();
void add_track(std::uintptr_t mainwindow, rust::String id, rust::String title, rust::String artists);
#ifdef __cplusplus
}
#endif
