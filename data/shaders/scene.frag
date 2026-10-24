#version 300 es
precision highp float;
// Ape Race's light (T34): the sky's colour from above and the ground's
// from below, a sun whose light wraps part way round each faceted shape,
// and haze in the place's own colour that closes in with distance. A
// glowing solid keeps its own colour; the glow pass adds light, softest at
// a shape's outline. Every number a look depends on is a uniform the core
// sends (Builder D3): the weights once, the colours with each view.
in vec3 vNormal;
in vec3 vWorld;
in vec4 vColor;
uniform float uPass;
uniform vec3 uEye;
uniform vec3 uFogColour;
uniform vec2 uFog;
uniform vec3 uSkyLight;
uniform vec3 uGroundLight;
uniform vec3 uSun;
uniform vec3 uSunDir;
uniform vec4 uMix;
uniform vec2 uGlow;
uniform vec3 uInkColour;
out vec4 outColor;
void main() {
  vec3 n = normalize(vNormal);
  if (!gl_FrontFacing) n = -n;
  float f = smoothstep(uFog.x, uFog.y, length(vWorld - uEye));
  if (uPass > 1.0) {
    outColor = vec4(mix(uInkColour, uFogColour, f), 1.0);
    return;
  }
  if (uPass > 0.0) {
    float facing = abs(dot(n, normalize(uEye - vWorld)));
    outColor = vec4(vColor.rgb * vColor.a * facing * facing * uGlow.x * (1.0 - f), 1.0);
    return;
  }
  float ndl = dot(n, uSunDir);
  float wrapped = clamp(ndl * uMix.w + uMix.w, 0.0, 1.0);
  vec3 ambient = mix(uGroundLight, uSkyLight, n.y * uMix.w + uMix.w);
  vec3 lit = vColor.rgb * (ambient * uMix.x + uSun * (max(ndl, 0.0) * uMix.y + wrapped * uMix.z));
  lit = mix(lit, vColor.rgb * uGlow.y, min(vColor.a, 1.0));
  outColor = vec4(mix(lit, uFogColour, f), 1.0);
}
