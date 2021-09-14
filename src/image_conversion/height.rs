use std::{
    sync::{Arc, Mutex},
    thread,
};

use image::{GrayImage, ImageBuffer, Luma, RgbImage};

use crate::image_conversion::sampler;

/// Converts a normal map to a bump map. Inspired by this paper:
/// https://doi.org/10.1145/2037826.2037839
pub fn normal_to_height(normal: RgbImage) -> GrayImage {
    let width = normal.width();
    let height = normal.height();
    // Note only x (red) and y (green) are used
    let mut depth_difference_map_x: ImageBuffer<Luma<f32>, Vec<f32>> =
        ImageBuffer::new(width, height);
    let mut depth_difference_map_y: ImageBuffer<Luma<f32>, Vec<f32>> =
        ImageBuffer::new(width, height);
    for x in 0..width {
        for y in 0..height {
            let normal_x = u8_to_float(normal.get_pixel(x, y)[0]);
            let normal_y = u8_to_float(normal.get_pixel(x, y)[1]);
            // transform from [0; 1] to [-1; 1]
            let normal_x = (normal_x * 2.0) - 1.0;
            let normal_y = (normal_y * 2.0) - 1.0;
            // Calculate height (see figure 1 in linked paper)
            let angle_x = normal_x * (std::f32::consts::PI / 2.0);
            let angle_y = normal_y * (std::f32::consts::PI / 2.0);
            let height_difference_x = f32::tan(-1.0 * angle_x);
            depth_difference_map_x.put_pixel(x, y, [height_difference_x].into());
            let height_difference_y = f32::tan(-1.0 * angle_y);
            depth_difference_map_y.put_pixel(x, y, [height_difference_y].into());
        }
    }
    // there are often a couple of outliers in the depth difference map
    // so clamp to a maximum that is that of the 95 percentile
    const PERCENTILE: f32 = 0.95;
    clamp_values_to_percentile(&mut depth_difference_map_x, PERCENTILE);
    clamp_values_to_percentile(&mut depth_difference_map_y, PERCENTILE);
    // DEBUG
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for x in 0..width {
        for y in 0..height {
            let v = depth_difference_map_x.get_pixel(x, y)[0];
            lo = f32::min(lo, v);
            hi = f32::max(hi, v);
        }
    }
    let mut output = GrayImage::new(width, height);
    for x in 0..width {
        for y in 0..height {
            let out = f32::abs(depth_difference_map_x.get_pixel(x, y)[0]);
            let out = (out - lo) / (hi - lo);
            output.put_pixel(x, y, [float_to_u8(out)].into());
        }
    }
    //return output;
    // integrate height for one pixel via integrating along rays spread out evenly in 360°
    // the result of all rays is averaged and denotes the height * -1
    const NUM_RAYS: u32 = 50;
    const RAY_LENGTH_PROPORTION_OF_WIDTH: f32 = 0.004;
    let ray_length_texel =
        (f32::max(width as f32, height as f32) * RAY_LENGTH_PROPORTION_OF_WIDTH).ceil() as u32;
    let heights: ImageBuffer<Luma<f32>, Vec<f32>> = ImageBuffer::new(width, height);
    let heights = Arc::new(Mutex::new(heights));
    {
        // use bilinear filtering of the DDM
        let sampler_ddm_x = Arc::new(sampler::Linear::new(depth_difference_map_x));
        let sampler_ddm_y = Arc::new(sampler::Linear::new(depth_difference_map_y));
        let num_threads = num_cpus::get() as u32;
        let mut handles = Vec::with_capacity(num_threads as usize);
        for thread_id in 0..num_threads {
            // clone the atomic references so that each threads has its own reference
            let heights = heights.clone();
            let sampler_ddm_x = sampler_ddm_x.clone();
            let sampler_ddm_y = sampler_ddm_y.clone();
            // start filtering in separate threads
            handles.push(thread::spawn(move || {
                let columns_per_thread = width / num_threads;
                let start = columns_per_thread * thread_id;
                let mut end = columns_per_thread * (thread_id + 1);
                if thread_id == num_threads - 1 {
                    //last thread has to do the rest
                    let rest = width % num_threads;
                    end += rest;
                }
                // filter
                for x in start..end {
                    for y in 0..height {
                        let mut sum = 0.0;
                        for ray in 0..NUM_RAYS {
                            // calculate the 2D direction vector of the ray
                            let angle =
                                ray as f32 * ((2.0 * std::f32::consts::PI) / (NUM_RAYS as f32));
                            let dir_x = f32::sin(angle);
                            let dir_y = f32::cos(angle);
                            // march the ray through the image with texture-edges clamped
                            // note that the current texel of the DDM should not be taken into account
                            let mut ray_sum = 0.0;
                            for i in 0..ray_length_texel {
                                let texel_x =
                                    (x as f32 + (i as f32 * dir_x)).clamp(0.0, width as f32 - 1.0);
                                let texel_y =
                                    (y as f32 + (i as f32 * dir_y)).clamp(0.0, height as f32 - 1.0);
                                let value_x = sampler_ddm_x.get_pixel(texel_x, texel_y);
                                let value_y = sampler_ddm_y.get_pixel(texel_x, texel_y);
                                ray_sum += ((dir_x * value_x) + (-1.0 * dir_y * value_y)) / 2.0;
                            }
                            sum += ray_sum / (ray_length_texel as f32 * 2.0 + 1.0);
                        }
                        sum = sum / NUM_RAYS as f32;
                        {
                            heights.lock().unwrap().put_pixel(x, y, [-1.0 * sum].into());
                        }
                    }
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
    }
    // unlock mutex, since here are no further threads running
    let heights = heights.lock().unwrap();
    // get lower and upper bounds of values
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for x in 0..width {
        for y in 0..height {
            let v = heights.get_pixel(x, y)[0];
            lo = f32::min(lo, v);
            hi = f32::max(hi, v);
        }
    }
    // convert to u8 gray-scale image
    let mut output = GrayImage::new(width, height);
    for x in 0..width {
        for y in 0..height {
            let out = heights.get_pixel(x, y)[0];
            // scale height into range [0; 1]
            let out = (out - lo) / (hi - lo);
            output.put_pixel(x, y, [float_to_u8(out)].into());
        }
    }
    output
}
/// Clamp the values of the image to values that lie in the given percentile
fn clamp_values_to_percentile(image: &mut ImageBuffer<Luma<f32>, Vec<f32>>, percentile: f32) {
    let mut list: Vec<f32> = image
        .enumerate_pixels()
        .map(|(_x, _y, pixel)| pixel[0])
        .collect();
    list.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // get clamp values
    let clamp_negative = list[((1.0 - percentile) * list.len() as f32) as usize];
    let clamp_positive = list[(percentile * list.len() as f32) as usize];
    // apply clamping
    image.enumerate_pixels_mut().for_each(|(_x, _y, pixel)| {
        pixel.0 = [pixel.0[0].clamp(clamp_negative, clamp_positive)].into()
    });
}

/// Converts a byte into a float in the range [0-1]
fn u8_to_float(input: u8) -> f32 {
    input as f32 / 255.0
}
/// Converts float in range [0-1] into byte
fn float_to_u8(input: f32) -> u8 {
    (input * 255.0) as u8
}
