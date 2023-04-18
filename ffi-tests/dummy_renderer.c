#include "dummy_renderer.h"

void update_render_preview(unsigned int image_width, unsigned int image_height, float *rgb_data) {
  for (int i=0; i<image_height; ++i) {
    for (int j=0; j<image_width; ++j) {
      rgb_data[i*image_width + j] = 1.0;
    }
  }
}
