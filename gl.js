// The WebGL 2 renderer (from Mage Tower Battle's gl.js). It uploads what the core returns and draws it;
// every size, stride and offset comes from the core's numbers(), so this
// file holds no layout of its own. After Long Time Comin's gl.js, with
// viewport commands so the watch screen draws four fields in one frame.
// Passes: solid; glow (added, no depth writes). Each viewport sets its own
// light and haze from the view's values (T34).

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader));
  }
  return shader;
}

function link(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertexSource));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program));
  }
  return program;
}

// Returns null when the browser has no WebGL 2; the page then says so.
export function makeRenderer(canvas, N, shaders, meshData, uniformValues) {
  const gl = canvas.getContext("webgl2", { antialias: true, powerPreference: "high-performance" });
  if (!gl) return null;

  const info = gl.getExtension("WEBGL_debug_renderer_info");
  const name = info ? String(gl.getParameter(info.UNMASKED_RENDERER_WEBGL)) : "";
  const software = shaders.software.some((s) => name.includes(s));

  const program = link(gl, shaders.vertex, shaders.fragment);
  gl.useProgram(program);
  gl.enable(gl.DEPTH_TEST);
  gl.enable(gl.CULL_FACE);
  gl.enable(gl.SCISSOR_TEST);

  const vao = gl.createVertexArray();
  gl.bindVertexArray(vao);

  const meshes = N.meshes.map((m) => {
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, meshData(m.id), gl.STATIC_DRAW);
    return { buffer, vertices: m.vertices };
  });

  const attribute = ([attr, size, offset]) => ({ loc: gl.getAttribLocation(program, attr), size, offset });
  const vertexAttributes = N.vertex.attributes.map(attribute);
  const instanceAttributes = N.instance.attributes.map(attribute);
  for (const a of vertexAttributes.concat(instanceAttributes)) gl.enableVertexAttribArray(a.loc);
  for (const a of instanceAttributes) gl.vertexAttribDivisor(a.loc, 1);

  const uniform = (u) => gl.getUniformLocation(program, u);
  let at = 0;
  for (const [u, size] of N.uniforms) {
    const values = uniformValues.subarray(at, at + size);
    gl[`uniform${size}fv`](uniform(u), values);
    at += size;
  }
  const uVP = uniform("uVP");
  const viewUniforms = N.view.uniforms.map(([u, start, size]) => ({ loc: uniform(u), start, size }));
  const uPass = uniform("uPass");
  const V = N.view;
  const P = N.passes;
  const O = N.ops;
  const buffers = [gl.createBuffer(), gl.createBuffer()];

  function enterPass(pass) {
    gl.uniform1f(uPass, pass);
    if (pass === P.glow) {
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.ONE, gl.ONE);
      gl.depthMask(false);
      gl.cullFace(gl.BACK);
      return;
    }
    gl.disable(gl.BLEND);
    gl.depthMask(true);
    // the ink pass draws back faces pushed out: the outline
    gl.cullFace(pass === P.ink ? gl.FRONT : gl.BACK);
  }

  return {
    software,
    setStatic(values) {
      gl.bindBuffer(gl.ARRAY_BUFFER, buffers[N.buffers.static]);
      gl.bufferData(gl.ARRAY_BUFFER, values, gl.STATIC_DRAW);
    },
    resize(width, height) {
      canvas.width = width;
      canvas.height = height;
    },
    frame(views, values, commands) {
      gl.bindBuffer(gl.ARRAY_BUFFER, buffers[N.buffers.dynamic]);
      gl.bufferData(gl.ARRAY_BUFFER, values, gl.STREAM_DRAW);
      gl.scissor(0, 0, canvas.width, canvas.height);
      gl.clearColor(0, 0, 0, 1);
      gl.depthMask(true);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      let current = null;
      for (let i = 0; i < commands.length; i += N.command_values) {
        const [op, a, b, c, d, e] = commands.subarray(i, i + N.command_values);
        if (op === O.viewport) {
          gl.viewport(a, b, c, d);
          gl.scissor(a, b, c, d);
          const [r, g, bl] = views.subarray(e + V.clear[0], e + V.clear[1]);
          gl.clearColor(r, g, bl, 1);
          gl.depthMask(true);
          gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
          gl.uniformMatrix4fv(uVP, false, views.subarray(e + V.matrix[0], e + V.matrix[1]));
          for (const v of viewUniforms) {
            gl[`uniform${v.size}fv`](v.loc, views.subarray(e + v.start, e + v.start + v.size));
          }
          current = null;
          continue;
        }
        // a draw: buffer slot, mesh, byte offset, count, pass
        if (e !== current) {
          enterPass(e);
          current = e;
        }
        const mesh = meshes[b];
        gl.bindBuffer(gl.ARRAY_BUFFER, mesh.buffer);
        for (const v of vertexAttributes) {
          gl.vertexAttribPointer(v.loc, v.size, gl.FLOAT, false, N.vertex.stride, v.offset);
        }
        gl.bindBuffer(gl.ARRAY_BUFFER, buffers[a]);
        for (const v of instanceAttributes) {
          gl.vertexAttribPointer(v.loc, v.size, gl.FLOAT, false, N.instance.stride, v.offset + c);
        }
        gl.drawArraysInstanced(gl.TRIANGLES, 0, mesh.vertices, d);
      }
      enterPass(P.solid);
    },
  };
}
