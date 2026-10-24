#version 300 es
// One shape (from Mage Tower Battle's scene.vert): the mesh vertex placed by the instance's three columns and
// translation (iA, iB, iC; translation in .w), coloured by iD (rgb, glow).
// After Long Time Comin's scene.vert; the world position goes on for the
// haze.
in vec3 aPos;
in vec3 aNormal;
in vec4 iA;
in vec4 iB;
in vec4 iC;
in vec4 iD;
uniform mat4 uVP;
uniform float uEps;
uniform float uPass;
uniform float uInk;
out vec3 vNormal;
out vec3 vWorld;
out vec4 vColor;
void main() {
  vec3 w = iA.xyz * aPos.x + iB.xyz * aPos.y + iC.xyz * aPos.z + vec3(iA.w, iB.w, iC.w);
  // the columns carry a non-uniform scale, so normals take the inverse scale
  vec3 n = iA.xyz * (aNormal.x / max(dot(iA.xyz, iA.xyz), uEps))
         + iB.xyz * (aNormal.y / max(dot(iB.xyz, iB.xyz), uEps))
         + iC.xyz * (aNormal.z / max(dot(iC.xyz, iC.xyz), uEps));
  vNormal = n;
  vWorld = w;
  vColor = iD;
  // the ink pass: pushed out along the normal, for the outline
  if (uPass > 1.0) w += normalize(n) * uInk;
  vWorld = w;
  gl_Position = uVP * vec4(w, 1.0);
}
