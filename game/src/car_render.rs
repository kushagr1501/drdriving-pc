//! Detailed, indexed car meshes with source normals, per-material maps and environment lighting.
use macroquad::miniquad::{
    BlendFactor, BlendState, BlendValue, Comparison, CullFace, Equation, FrontFaceOrder,
    PipelineParams, UniformDesc, UniformType,
};
use macroquad::models::{draw_mesh, Mesh, Vertex};
use macroquad::prelude::*;

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
uniform mat4 Model;
uniform mat4 Projection;
varying highp vec3 world;
varying highp vec3 nrm;
varying highp vec3 tangent;
varying highp float tangentSign;
varying highp vec2 uv;
void main() {
    vec4 p = Model * vec4(position, 1.0);
    world = p.xyz;
    nrm = normalize(mat3(Model) * normal.xyz);
    tangent = normalize(mat3(Model) * (color0.xyz / 255.0 * 2.0 - 1.0));
    tangentSign = normal.w;
    uv = texcoord;
    gl_Position = Projection * p;
}"#;

const FRAGMENT: &str = r#"#version 100
precision highp float;
varying highp vec3 world;
varying highp vec3 nrm;
varying highp vec3 tangent;
varying highp float tangentSign;
varying highp vec2 uv;
uniform sampler2D Texture;
uniform sampler2D SurfaceMap;
uniform sampler2D NormalMap;
uniform sampler2D EmissionMap;
uniform sampler2D OcclusionMap;
uniform sampler2D Environment;
uniform sampler2D EnvironmentBlur;
uniform vec3 Eye;
uniform vec4 BaseColor;
uniform vec4 Surface;
uniform vec4 MapFlags;
uniform vec4 Extra;
uniform vec3 Emission;
uniform float Exposure;
const float PI = 3.14159265359;
vec3 linearColor(vec3 c) { return pow(max(c,vec3(0.0)),vec3(2.2)); }
vec3 environment(vec3 direction, float roughness) {
    vec2 e = vec2(atan(direction.z,direction.x)/(2.0*PI)+0.5, acos(clamp(direction.y,-1.0,1.0))/PI);
    vec3 sharp = linearColor(texture2D(Environment,e).rgb);
    vec3 blurred = linearColor(texture2D(EnvironmentBlur,e).rgb);
    return mix(sharp,blurred,smoothstep(0.02,0.22,roughness));
}
vec3 fresnel(float c, vec3 f0) { return f0+(1.0-f0)*pow(1.0-clamp(c,0.0,1.0),5.0); }
vec3 directLight(vec3 N,vec3 V,vec3 L,vec3 color,vec3 base,float metal,float rough,vec3 f0) {
    vec3 H=normalize(V+L);
    float nl=max(dot(N,L),0.0),nv=max(dot(N,V),0.001),nh=max(dot(N,H),0.0),vh=max(dot(V,H),0.0);
    float a=rough*rough,a2=a*a,d=nh*nh*(a2-1.0)+1.0;
    float D=a2/max(PI*d*d,0.001);
    float k=(rough+1.0)*(rough+1.0)/8.0;
    float G=(nl/(nl*(1.0-k)+k))*(nv/(nv*(1.0-k)+k));
    vec3 F=fresnel(vh,f0);
    // A finite softbox highlight avoids sub-pixel specular sparkle on dense rims.
    vec3 spec=min(D*G*F/max(4.0*nl*nv,0.002),vec3(0.65));
    return ((1.0-F)*(1.0-metal)*base/PI+spec)*color*nl;
}
void main() {
    vec4 sampleColor=texture2D(Texture,uv);
    if(Extra.y>0.0 && sampleColor.a*BaseColor.a<Extra.y)discard;
    vec3 base=linearColor(sampleColor.rgb)*BaseColor.rgb;
    vec3 N=normalize(nrm);
    // Source normals already point outward, including mirrored source nodes.
    if(MapFlags.y>0.5) {
        vec3 T=normalize(tangent-N*dot(tangent,N));
        vec3 B=cross(N,T)*tangentSign;
        vec3 detail=texture2D(NormalMap,uv).xyz*2.0-1.0;
        detail.xy*=Extra.x;
        N=normalize(mat3(T,B,N)*detail);
    }
    vec3 orm=texture2D(SurfaceMap,uv).rgb;
    float rough=clamp(Surface.y*mix(1.0,orm.g,MapFlags.x),0.055,0.96);
    float metal=clamp(Surface.x*mix(1.0,orm.b,MapFlags.x),0.0,1.0);
    if(Surface.z>0.95)rough=max(rough,0.28);
    if(Extra.w>0.5) {
        float rubber=1.0-smoothstep(0.025,0.10,max(base.r,max(base.g,base.b)));
        rough=mix(max(rough,0.22),0.82,rubber);
        metal=mix(max(metal,0.60),0.0,rubber);
        base=mix(base,max(base,vec3(0.006)),rubber);
    }
    float ao=mix(1.0,texture2D(OcclusionMap,uv).r,MapFlags.z);
    vec3 V=normalize(Eye-world);
    // Imported double-sided and mirrored meshes may have inward-facing normals.
    // Use the visible hemisphere, avoiding a full white grazing reflection.
    if(dot(N,V)<0.0)N=-N;
    vec3 R=reflect(-V,N);
    float nv=max(dot(N,V),0.001);
    vec3 f0=mix(vec3(0.04),base,metal),F=fresnel(nv,f0);
    vec3 hemi=mix(vec3(0.13,0.15,0.18),vec3(0.65,0.73,0.86),N.y*0.5+0.5);
    vec3 diffuse=base*(1.0-metal)*(0.30+hemi*0.50);
    vec3 reflected=environment(R,rough);
    vec3 specular=reflected*(F*(1.0-rough*0.85)+vec3(0.005))*1.15;
    vec3 radiance=(diffuse+specular)*ao;
    radiance+=directLight(N,V,normalize(vec3(-0.5,0.9,0.6)),vec3(2.3,2.13,1.95),base,metal,rough,f0);
    radiance+=directLight(N,V,normalize(vec3(0.8,0.4,-0.5)),vec3(0.58,0.7,0.95),base,metal,rough,f0);
    float coat=Surface.z*(1.0-step(0.5,Extra.w));
    vec3 coatF=min(fresnel(nv,vec3(0.045)),vec3(0.22));
    radiance=radiance*(1.0-coat*coatF)+environment(R,0.20)*coatF*coat*1.15;
    // Glass keeps readable silhouettes and reflects the studio/sky rather than a flat dark fill.
    if(Surface.w>0.5)radiance=mix(base*0.22+hemi*0.025,environment(R,0.24)*0.65,0.24+pow(1.0-nv,3.0)*0.56);
    radiance+=Emission*mix(vec3(1.0),linearColor(texture2D(EmissionMap,uv).rgb),MapFlags.w);
    radiance*=Exposure;
    vec3 mapped=(radiance*(2.51*radiance+0.03))/(radiance*(2.43*radiance+0.59)+0.14);
    float alpha=Surface.w>0.5?0.98:1.0;
    gl_FragColor=vec4(pow(clamp(mapped,0.0,1.0),vec3(1.0/2.2)),alpha*Extra.z);
}"#;

pub struct CarRenderer {
    opaque: Material,
    glass: Material,
    white: Texture2D,
    normal: Texture2D,
    environment: Texture2D,
    blurred: Texture2D,
    shadow: Texture2D,
}

fn shader(glass: bool) -> Material {
    load_material(
        ShaderSource::Glsl {
            vertex: VERTEX,
            fragment: FRAGMENT,
        },
        MaterialParams {
            pipeline_params: PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                cull_face: CullFace::Nothing,
                front_face_order: FrontFaceOrder::CounterClockwise,
                color_blend: glass.then(|| {
                    BlendState::new(
                        Equation::Add,
                        BlendFactor::Value(BlendValue::SourceAlpha),
                        BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                    )
                }),
                ..Default::default()
            },
            uniforms: vec![
                UniformDesc::new("Eye", UniformType::Float3),
                UniformDesc::new("BaseColor", UniformType::Float4),
                UniformDesc::new("Surface", UniformType::Float4),
                UniformDesc::new("MapFlags", UniformType::Float4),
                UniformDesc::new("Extra", UniformType::Float4),
                UniformDesc::new("Emission", UniformType::Float3),
                UniformDesc::new("Exposure", UniformType::Float1),
            ],
            textures: [
                "SurfaceMap",
                "NormalMap",
                "EmissionMap",
                "OcclusionMap",
                "Environment",
                "EnvironmentBlur",
            ]
            .iter()
            .map(|v| v.to_string())
            .collect(),
        },
    )
    .expect("Detailed car shader")
}

impl CarRenderer {
    pub async fn load(base: &str) -> Self {
        let white = Texture2D::from_rgba8(1, 1, &[255, 255, 255, 255]);
        let normal = Texture2D::from_rgba8(1, 1, &[128, 128, 255, 255]);
        let environment = Texture2D::from_file_with_format(
            &crate::assets::bytes(base, "envi/car-reflections.png").await,
            Some(ImageFormat::Png),
        );
        let blurred = Texture2D::from_file_with_format(
            &crate::assets::bytes(base, "envi/car-reflections-blur.png").await,
            Some(ImageFormat::Png),
        );
        let mut pixels = Vec::with_capacity(128 * 128 * 4);
        for y in 0..128 {
            for x in 0..128 {
                let r = ((x as f32 - 63.5) / 63.5).powi(2) + ((y as f32 - 63.5) / 63.5).powi(2);
                let alpha = ((1.0 - r).max(0.0).powf(1.8) * 180.0) as u8;
                pixels.extend_from_slice(&[10, 13, 18, alpha]);
            }
        }
        Self {
            opaque: shader(false),
            glass: shader(true),
            white,
            normal,
            environment,
            blurred,
            shadow: Texture2D::from_rgba8(128, 128, &pixels),
        }
    }

    pub fn shadow(&self, model: Mat4, width: f32, length: f32) {
        let p = [
            vec3(-width * 0.62, 0.012, -length * 0.58),
            vec3(width * 0.62, 0.012, -length * 0.58),
            vec3(width * 0.62, 0.012, length * 0.58),
            vec3(-width * 0.62, 0.012, length * 0.58),
        ];
        let uv = [
            vec2(0.0, 0.0),
            vec2(1.0, 0.0),
            vec2(1.0, 1.0),
            vec2(0.0, 1.0),
        ];
        let vertices = (0..4)
            .map(|i| Vertex::new2(model.transform_point3(p[i]), uv[i], WHITE))
            .collect();
        draw_mesh(&Mesh {
            vertices,
            indices: vec![0, 2, 1, 0, 3, 2],
            texture: Some(self.shadow.clone()),
        });
    }
}

pub struct DetailedCar {
    parts: Vec<Part>,
    materials: Vec<Surface>,
    textures: Vec<Texture2D>,
    wheels: Vec<(Vec3, bool)>,
    pub name: String,
    pub triangles: usize,
    pub width: f32,
    pub length: f32,
}
struct Part {
    chunks: Vec<Mesh>,
    material: usize,
    wheel: Option<usize>,
}
struct Surface {
    base: Vec4,
    params: Vec4,
    flags: Vec4,
    extra: Vec4,
    emission: Vec3,
    images: [Option<usize>; 5],
}

impl DetailedCar {
    pub fn wheel_count(&self) -> usize {
        self.wheels.len()
    }
    pub fn draw(
        &self,
        renderer: &CarRenderer,
        transform: Mat4,
        eye: Vec3,
        spin: f32,
        steering: f32,
        alpha: f32,
    ) {
        for transparent in [false, true] {
            let shader = if transparent {
                &renderer.glass
            } else {
                &renderer.opaque
            };
            gl_use_material(shader);
            shader.set_uniform("Eye", eye);
            shader.set_uniform("Exposure", 1.0f32);
            shader.set_texture("Environment", renderer.environment.clone());
            shader.set_texture("EnvironmentBlur", renderer.blurred.clone());
            for part in &self.parts {
                let surface = &self.materials[part.material];
                if (surface.params.w > 0.5) != transparent {
                    continue;
                }
                // Macroquad snapshots uniforms per draw, but additional sampler bindings
                // belong to the shared pipeline. Submit before changing material maps.
                unsafe {
                    get_internal_gl().flush();
                }
                shader.set_uniform("BaseColor", surface.base);
                shader.set_uniform("Surface", surface.params);
                shader.set_uniform("MapFlags", surface.flags);
                shader.set_uniform(
                    "Extra",
                    vec4(
                        surface.extra.x,
                        surface.extra.y,
                        alpha,
                        part.wheel.is_some() as u8 as f32,
                    ),
                );
                shader.set_uniform("Emission", surface.emission);
                let texture = |id: Option<usize>, fallback: &Texture2D| {
                    id.map(|i| self.textures[i].clone())
                        .unwrap_or_else(|| fallback.clone())
                };
                shader.set_texture("SurfaceMap", texture(surface.images[2], &renderer.white));
                shader.set_texture("NormalMap", texture(surface.images[1], &renderer.normal));
                shader.set_texture("OcclusionMap", texture(surface.images[3], &renderer.white));
                shader.set_texture("EmissionMap", texture(surface.images[4], &renderer.white));
                let mut matrix = transform;
                if let Some(i) = part.wheel {
                    let (pivot, front) = self.wheels[i];
                    matrix = matrix
                        * Mat4::from_translation(pivot)
                        * Mat4::from_rotation_y(if front { steering } else { 0.0 })
                        * Mat4::from_rotation_x(spin)
                        * Mat4::from_translation(-pivot);
                }
                unsafe {
                    get_internal_gl().quad_gl.push_model_matrix(matrix);
                }
                for chunk in &part.chunks {
                    draw_mesh(chunk);
                }
                unsafe {
                    get_internal_gl().quad_gl.pop_model_matrix();
                }
            }
            unsafe {
                get_internal_gl().flush();
            }
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::DetailedCar;
    use serde_json::{json, Value};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn package(header: Value, blob: &[u8]) -> Vec<u8> {
        let json = serde_json::to_vec(&header).unwrap();
        let mut data = b"DRHQ0001".to_vec();
        data.extend_from_slice(&(json.len() as u32).to_le_bytes());
        data.extend_from_slice(&json);
        data.extend_from_slice(blob);
        data
    }

    fn rejects(data: &[u8], message: &str) {
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "drdriving-car-validation-{}-{}.carhq",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, data).unwrap();
        let result = DetailedCar::load(&path, 0);
        std::fs::remove_file(path).unwrap();
        match result {
            Err(e) => assert!(e.contains(message), "Expected {message}, received {e}"),
            Ok(_) => panic!("Malformed car was accepted"),
        }
    }

    #[test]
    fn rejects_truncated_or_foreign_packages() {
        rejects(b"other model", "Invalid detailed car package");
        let mut data = b"DRHQ0001".to_vec();
        data.extend_from_slice(&500u32.to_le_bytes());
        rejects(&data, "Invalid detailed car header");
    }

    #[test]
    fn rejects_unsupported_version_and_impossible_dimensions() {
        rejects(
            &package(json!({"version":2}), &[]),
            "Unsupported detailed car version",
        );
        rejects(
            &package(json!({"version":1,"bounds":[0,1,4]}), &[]),
            "Invalid car dimensions",
        );
    }

    #[test]
    fn rejects_image_buffers_outside_package() {
        rejects(
            &package(
                json!({"version":1,"bounds":[2,1.5,4],"images":[{"offset":4000,"length":32}]}),
                &[],
            ),
            "Buffer outside detailed car package",
        );
    }

    #[test]
    fn rejects_oversized_texture_before_decoding() {
        let mut image = vec![0u8; 24];
        image[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
        image[16..20].copy_from_slice(&4097u32.to_be_bytes());
        image[20..24].copy_from_slice(&1024u32.to_be_bytes());
        rejects(
            &package(
                json!({"version":1,"bounds":[2,1.5,4],"images":[{"offset":0,"length":24}]}),
                &image,
            ),
            "exceeds 4096 pixels",
        );
    }
}

impl DetailedCar {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &std::path::Path, slot: usize) -> Result<Self, String> {
        use std::io::Read;
        let mut data = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(96 * 1024 * 1024 + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        Self::from_bytes(&data, slot)
    }

    pub fn from_bytes(data: &[u8], slot: usize) -> Result<Self, String> {
        use serde_json::Value;
        if data.len() > 96 * 1024 * 1024 || data.len() < 12 || &data[..8] != b"DRHQ0001" {
            return Err("Invalid detailed car package".into());
        }
        let header_len = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        if header_len > 1024 * 1024 || header_len > data.len() - 12 {
            return Err("Invalid detailed car header".into());
        }
        let header: Value =
            serde_json::from_slice(&data[12..12 + header_len]).map_err(|e| e.to_string())?;
        let blob = &data[12 + header_len..];
        let array = |key: &str| {
            header[key]
                .as_array()
                .ok_or_else(|| format!("Missing {key}"))
        };
        if header["version"] != 1 {
            return Err("Unsupported detailed car version".into());
        }
        let bounds = array("bounds")?;
        let number = |v: &Value| {
            v.as_f64()
                .map(|n| n as f32)
                .filter(|n| n.is_finite())
                .ok_or_else(|| "Invalid numeric value".to_string())
        };
        if bounds.len() != 3 {
            return Err("Invalid bounds".into());
        }
        let dimensions = vec3(
            number(&bounds[0])?,
            number(&bounds[1])?,
            number(&bounds[2])?,
        );
        if dimensions.min_element() < 0.01 || dimensions.max_element() > 30.0 {
            return Err("Invalid car dimensions".into());
        }
        let info = drdriving_core::cars::car(slot);
        let scale = (info.half_width * 2.0 / dimensions.x)
            .min(info.half_length * 2.0 / dimensions.z)
            .min(info.eye_height * 1.35 / dimensions.y);
        let slice = |v: &Value| -> Result<&[u8], String> {
            let offset = v["offset"].as_u64().ok_or("Missing offset")? as usize;
            let length = v["length"].as_u64().ok_or("Missing length")? as usize;
            let end = offset.checked_add(length).ok_or("Buffer length overflow")?;
            blob.get(offset..end)
                .ok_or_else(|| "Buffer outside detailed car package".to_string())
        };
        let mut textures = Vec::new();
        let mut texture_bytes = 0u64;
        if array("images")?.len() > 80 {
            return Err("Too many car textures".into());
        }
        for image in array("images")? {
            let bytes = slice(image)?;
            if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
                return Err("Invalid car texture".into());
            }
            let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            if w == 0 || h == 0 || w > 4096 || h > 4096 {
                return Err("Detailed car texture exceeds 4096 pixels".into());
            }
            texture_bytes += w as u64 * h as u64 * 4;
            if texture_bytes > 256 * 1024 * 1024 {
                return Err("Detailed car textures exceed memory limit".into());
            }
            let image = Image::from_file_with_format(bytes, Some(ImageFormat::Png))
                .map_err(|e| e.to_string())?;
            let tex = Texture2D::from_image(&image);
            tex.set_filter(FilterMode::Linear);
            unsafe {
                use macroquad::miniquad::{MipmapFilterMode, TextureWrap};
                let gl = get_internal_gl();
                let id = tex.raw_miniquad_id();
                if w.is_power_of_two() && h.is_power_of_two() {
                    gl.quad_context
                        .texture_set_wrap(id, TextureWrap::Repeat, TextureWrap::Repeat);
                    gl.quad_context.texture_generate_mipmaps(id);
                    gl.quad_context.texture_set_filter(
                        id,
                        FilterMode::Linear,
                        MipmapFilterMode::Linear,
                    );
                } else {
                    // WebGL 1 requires clamp-to-edge and no mipmaps for NPOT textures.
                    gl.quad_context
                        .texture_set_wrap(id, TextureWrap::Clamp, TextureWrap::Clamp);
                }
            }
            textures.push(tex);
        }
        let vec = |v: &Value, n: usize| -> Result<Vec<f32>, String> {
            let a = v.as_array().ok_or("Expected vector")?;
            if a.len() != n {
                return Err("Invalid vector length".into());
            }
            a.iter().map(number).collect()
        };
        let mut materials = Vec::new();
        for m in array("materials")? {
            let b = vec(&m["base"], 4)?;
            let e = vec(&m["emission"], 3)?;
            let mut images = [None; 5];
            for (i, key) in [
                "baseMap",
                "normalMap",
                "surfaceMap",
                "occlusionMap",
                "emissionMap",
            ]
            .iter()
            .enumerate()
            {
                let index = m[*key].as_i64().ok_or("Invalid material texture index")?;
                if index >= 0 {
                    if index as usize >= textures.len() {
                        return Err("Material texture index out of bounds".into());
                    }
                    images[i] = Some(index as usize);
                }
            }
            materials.push(Surface {
                base: Vec4::from_slice(&b),
                params: vec4(
                    number(&m["metallic"])?.clamp(0.0, 1.0),
                    number(&m["roughness"])?.clamp(0.04, 1.0),
                    number(&m["clearcoat"])?.clamp(0.0, 1.0),
                    number(&m["glass"])?.clamp(0.0, 1.0),
                ),
                flags: vec4(
                    images[2].is_some() as u8 as f32,
                    images[1].is_some() as u8 as f32,
                    images[3].is_some() as u8 as f32,
                    images[4].is_some() as u8 as f32,
                ),
                extra: vec4(
                    number(&m["normalScale"])?.clamp(0.0, 3.0),
                    number(&m["alphaCutoff"])?.clamp(0.0, 1.0),
                    1.0,
                    0.0,
                ),
                emission: Vec3::from_slice(&e),
                images,
            });
        }
        if materials.is_empty() || materials.len() > 128 {
            return Err("Invalid car materials".into());
        }
        let mut wheels = Vec::new();
        for w in array("wheels")? {
            let p = vec(&w["pivot"], 3)?;
            wheels.push((
                Vec3::from_slice(&p) * scale,
                w["front"].as_bool().ok_or("Invalid axle")?,
            ));
        }
        if wheels.len() != 4 {
            return Err("Detailed car needs four wheel pivots".into());
        }
        let white = Texture2D::from_rgba8(1, 1, &[255, 255, 255, 255]);
        let mut parts = Vec::new();
        let mut triangles = 0usize;
        if array("parts")?.len() > 256 {
            return Err("Too many car mesh groups".into());
        }
        for p in array("parts")? {
            let vb = slice(&p["vertices"])?;
            let ib = slice(&p["indices"])?;
            if vb.len() % 48 != 0 || ib.len() % 12 != 0 {
                return Err("Invalid mesh buffers".into());
            }
            let count = vb.len() / 48;
            if count > 500_000 {
                return Err("Car mesh too large".into());
            }
            triangles += ib.len() / 12;
            if triangles > 600_000 {
                return Err("Car exceeds triangle limit".into());
            }
            let material = p["material"].as_u64().ok_or("Missing material")? as usize;
            if material >= materials.len() {
                return Err("Mesh material out of bounds".into());
            }
            let wheel = p["wheel"].as_i64().ok_or("Missing wheel group")?;
            if !(-1..=3).contains(&wheel) {
                return Err("Invalid wheel group".into());
            }
            let texture = materials[material].images[0]
                .map(|i| textures[i].clone())
                .unwrap_or_else(|| white.clone());
            let mut vertices = Vec::with_capacity(count);
            for record in vb.chunks_exact(48) {
                let mut f = [0.0f32; 12];
                for (i, b) in record.chunks_exact(4).enumerate() {
                    f[i] = f32::from_le_bytes(b.try_into().unwrap());
                }
                if f.iter().any(|n| !n.is_finite()) {
                    return Err("Non-finite mesh coordinate".into());
                }
                let mut v = Vertex::new2(vec3(f[0], f[1], f[2]) * scale, vec2(f[6], f[7]), WHITE);
                v.normal = vec4(f[3], f[4], f[5], f[11]);
                v.color = [
                    ((f[8] * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8,
                    ((f[9] * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8,
                    ((f[10] * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0) as u8,
                    255,
                ];
                vertices.push(v);
            }
            let mut chunks = Vec::new();
            let mut chunk_vertices = Vec::new();
            let mut chunk_indices = Vec::new();
            let mut remap = std::collections::HashMap::new();
            for index in ib.chunks_exact(4) {
                let index = u32::from_le_bytes(index.try_into().unwrap()) as usize;
                if index >= vertices.len() {
                    return Err("Mesh vertex index out of bounds".into());
                }
                if chunk_indices.len() == 4500 {
                    chunks.push(Mesh {
                        vertices: std::mem::take(&mut chunk_vertices),
                        indices: std::mem::take(&mut chunk_indices),
                        texture: Some(texture.clone()),
                    });
                    remap.clear();
                }
                let next = chunk_vertices.len() as u16;
                let local = *remap.entry(index).or_insert_with(|| {
                    chunk_vertices.push(vertices[index]);
                    next
                });
                chunk_indices.push(local);
            }
            if !chunk_indices.is_empty() {
                chunks.push(Mesh {
                    vertices: chunk_vertices,
                    indices: chunk_indices,
                    texture: Some(texture),
                });
            }
            parts.push(Part {
                chunks,
                material,
                wheel: if wheel < 0 {
                    None
                } else {
                    Some(wheel as usize)
                },
            });
        }
        if triangles == 0 {
            return Err("Empty detailed car".into());
        }
        Ok(Self {
            parts,
            materials,
            textures,
            wheels,
            name: header["name"]
                .as_str()
                .unwrap_or("Custom car")
                .chars()
                .filter(|c| !c.is_control())
                .take(40)
                .collect(),
            triangles,
            width: dimensions.x * scale,
            length: dimensions.z * scale,
        })
    }
}
