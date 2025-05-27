#pragma once
#include <cstdint>
#include <cstddef>
#include <string>
#include "../../target/cxxbridge/rust/cxx.h"

#ifdef __cplusplus
extern "C" {
#endif
void show_widget_window(std::int32_t argc, std::int8_t** argv);
// Expose getter for the main window's MediaPlayer
std::uintptr_t get_mainwindow_mediaplayer();
// Expose setter for MediaPlayer progress
void mediaplayer_set_progress(std::uintptr_t mediaplayer, double value);
// Expose setter for MediaPlayer track title
void mediaplayer_set_track(std::uintptr_t mediaplayer, rust::cxxbridge1::String title);
#ifdef __cplusplus
}
#endif
