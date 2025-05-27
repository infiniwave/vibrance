#pragma once
#include <cstdint>
#include <cstddef>

#ifdef __cplusplus
extern "C" {
#endif
void show_widget_window(std::int32_t argc, std::int8_t** argv);
// Expose getter for the main window's MediaPlayer
std::uintptr_t get_mainwindow_mediaplayer();
// Expose setter for MediaPlayer progress
void mediaplayer_set_progress(std::uintptr_t mediaplayer, double value);
#ifdef __cplusplus
}
#endif
