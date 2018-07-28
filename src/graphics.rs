extern crate cgmath;
extern crate gl;
extern crate glutin;

use self::cgmath::prelude::*;
use self::cgmath::{Matrix4, Vector3};
use self::gl::types::*;
use self::glutin::{GlContext, GlWindow};
use super::{HEIGHT, WIDTH};
use std::ffi::{CStr, CString};
use std::mem;
use std::os::raw::c_void;
use std::ptr;

const VERTEX_SHADER: &str = r#"
#version 330 core
layout (location = 0) in vec3 position;
uniform mat4 translate;
void main() {
    gl_Position = translate * vec4(position, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 330 core
out vec4 fragment_colour;
void main() {
    fragment_colour = vec4(1.0, 1.0, 1.0, 1.0);
}
"#;

const X_UNIT: GLfloat = 2.0 / WIDTH as GLfloat;
const Y_UNIT: GLfloat = 2.0 / HEIGHT as GLfloat;

const VERTICES: [GLfloat; 12] = [
    // top left
    -1.0,
    1.0,
    0.0,
    // top right
    -1.0 + X_UNIT,
    1.0,
    0.0,
    // bottom right
    -1.0 + X_UNIT,
    1.0 - Y_UNIT,
    0.0,
    // bottom left
    -1.0,
    1.0 - Y_UNIT,
    0.0,
];
const INDICES: [GLint; 6] = [
    0, 1, 2, // first triangle
    2, 3, 0, // second triangle
];

#[derive(Default)]
pub struct Graphics {
    shader_program: GLuint,
}

impl Graphics {
    pub fn new() -> Graphics {
        Graphics::default()
    }

    pub fn init(&mut self, gl_window: &GlWindow) -> Result<(), String> {
        gl::load_with(|symbol| gl_window.get_proc_address(symbol) as *const _);

        unsafe {
            let mut success = GLint::from(gl::FALSE);
            let mut info_log = Vec::with_capacity(512);
            info_log.set_len(512 - 1);

            // Compile vertex shader.
            let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
            let c_str_vert = CString::new(VERTEX_SHADER).unwrap();
            gl::ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), ptr::null());
            gl::CompileShader(vertex_shader);

            // Check for vertex shader compilation errors.
            gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
            if success != GLint::from(gl::TRUE) {
                gl::GetShaderInfoLog(
                    vertex_shader,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                return Err(format!(
                    "vertex shader compilation failed: {}",
                    CStr::from_ptr(info_log.as_ptr()).to_string_lossy(),
                ));
            }

            // Compile fragment shader.
            let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_str_frag = CString::new(FRAGMENT_SHADER).unwrap();
            gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), ptr::null());
            gl::CompileShader(fragment_shader);

            // Check for fragment shader compilation errors.
            gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
            if success != GLint::from(gl::TRUE) {
                gl::GetShaderInfoLog(
                    fragment_shader,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                return Err(format!(
                    "fragment shader compilation failed: {}",
                    CStr::from_ptr(info_log.as_ptr()).to_string_lossy(),
                ));
            }

            // Link shader program.
            self.shader_program = gl::CreateProgram();
            gl::AttachShader(self.shader_program, vertex_shader);
            gl::AttachShader(self.shader_program, fragment_shader);
            gl::LinkProgram(self.shader_program);

            // Check for shader program linking errors.
            gl::GetProgramiv(self.shader_program, gl::LINK_STATUS, &mut success);
            if success != GLint::from(gl::TRUE) {
                gl::GetProgramInfoLog(
                    self.shader_program,
                    512,
                    ptr::null_mut(),
                    info_log.as_mut_ptr() as *mut GLchar,
                );
                return Err(format!(
                    "shader program compilation failed:\n{}",
                    CStr::from_ptr(info_log.as_ptr()).to_string_lossy(),
                ));
            }

            gl::UseProgram(self.shader_program);

            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
            gl::DeleteProgram(self.shader_program);

            let (mut vao, mut vbo, mut ebo) = (0, 0, 0);
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);
            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (VERTICES.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &VERTICES[0] as *const f32 as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (INDICES.len() * mem::size_of::<GLint>()) as GLsizeiptr,
                &INDICES[0] as *const i32 as *const c_void,
                gl::STATIC_DRAW,
            );

            gl::VertexAttribPointer(
                0,
                3,
                gl::FLOAT,
                gl::FALSE,
                3 * mem::size_of::<GLfloat>() as GLsizei,
                ptr::null(),
            );
            gl::EnableVertexAttribArray(0);

            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        Ok(())
    }

    pub fn clear_colour(&self, red: f32, green: f32, blue: f32, alpha: f32) {
        unsafe {
            gl::ClearColor(red, green, blue, alpha);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    pub fn draw_square_at(&self, x: usize, y: usize) {
        let translate = Matrix4::<f32>::from_translation(Vector3::<f32>::new(
            x as f32 * X_UNIT,
            y as f32 * -Y_UNIT,
            0.0,
        ));
        unsafe {
            // Unwrap is safe, because CString::new() only returns Err when a nul-byte is found.
            let translate_str = CString::new("translate").unwrap();
            let translate_uniform =
                gl::GetUniformLocation(self.shader_program, translate_str.as_ptr());
            gl::UniformMatrix4fv(translate_uniform, 1, gl::FALSE, translate.as_ptr());
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, ptr::null());
        }
    }
}
