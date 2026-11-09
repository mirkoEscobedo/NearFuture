# Presentation probe preparation (#100)

Status: experimental public-seam process fixture only. No installed-game frame,
late-game baseline, observer overhead, save/load or native lifecycle result is
qualified by this component. The accepted profile and all offload/writer flags
remain unchanged.

The observation question is how to measure elapsed rendering cadence separately
from the existing observer's own callback. Callback entry intervals do not prove
a whole-frame boundary, and JFR execution samples do not supply frame durations.
The selected experimental route is a startup-only Java agent at the public LWJGL
`Display.swapBuffers()` boundary. The [LWJGL API](https://legacy.lwjgl.org/javadoc/org/lwjgl/opengl/Display.html)
describes buffer swapping and its use by `update()`. The
[upstream 2.9.3 source](https://github.com/LWJGL/lwjgl/blob/lwjgl2.9.3/src/java/org/lwjgl/opengl/Display.java)
supports that interpretation; it does not attest the installed game loop.

The agent argument is the exact SHA-256 of the selected
`org/lwjgl/opengl/Display.class`. It accepts one definition with the expected
public static non-native `swapBuffers()V` method, adding a marker only on normal
returns. It does not attach, retransform, replace game decisions, or change the
callee's original effects. No game implementation or licensed binary is shipped.
ASM 9.7.1 is pinned by dependency lock and strict artifact SHA-256 verification.

The owned launch supplies a 32-character hexadecimal `NF_PERFORMANCE_TRACE_ID`
in the child environment. Optional `NF_PERFORMANCE_OBSERVER=true|false` selects
the existing callback observer. The startup edge sets their existing profiling
properties and refuses conflicting preconfigured values. Launcher files and
agent arguments stay fixed across repetitions; anonymous trace IDs do not alter
the runtime-manifest identity. Record the effective observer choice and JFR
settings separately with each run's instrumentation provenance. Environment
choices are explicit experiment inputs, not a way to hide semantic differences.

`nf.PresentInterval` contains the anonymous trace ID, a sequence number and actual
`System.nanoTime()` elapsed nanoseconds between successive successful markers.
The first observation supplies an anchor, not a measured interval. One rendering
thread and at most 100,000 recorded intervals are admitted per JVM. The marker
owns no game object and performs no file/socket work or waits. Wrong class pins,
malformed startup configuration, multiple definitions or rendering threads and
counter/clock failure reject observation. A rejection invalidates the run; it is
not zero-cost or missing-frame evidence.

One recording named `NF100` is admitted per JVM. Stopping or closing it
permanently ends observation; a second recording cannot resume the sequence,
including when no presentation happened during the gap. The probe emits its
effective anonymous trace, class pin and callback-observer choice as
`nf.ProbeConfiguration`. Any `nf.FrameProbeRejected` or `jdk.DataLoss` event
invalidates the recording. Exporters must discard partial output on failure.

`tools/performance/jfr/RuntimeEvidence.java` reads an owned recording offline.
Its inputs are the recording, expected trace, class pin and exact owned PID.
It requires recorded `jdk.JVMInformation`, one matching configuration and a
nonempty sequential presentation stream. It hashes the actual JVM arguments
and flags together, and JVM name/version separately. Each ordered UTF-8 field
is prefixed by its four-byte big-endian byte length. Raw arguments, paths and
runtime strings are not exported. These identities do not replace binary or
hardware inventory. Keep all JVM arguments, including the recording filename,
byte-identical within matched repetitions; archive completed recordings only
after the process exits. A missing or inconsistent identity refuses admission.

These intervals include application work, pacing and waits between presentations.
They are not CPU execution time or the duration of `swapBuffers()` itself. Before
using them as wall-frame observations, native evidence must establish the exact
class identity, one presentation per rendered frame in the measured scene,
thread/lifecycle behavior and clock uncertainty. Hidden/minimized windows or
scene transitions require exclusion or separate segmentation. Long intervals
must remain in the trace. Callback spans stay a distinct metric.

The exporter anchors each preceding interval to its marker's JFR start time;
the marker's own short event duration is never a frame duration. It exports
the observed monotonic-to-marker uncertainty plus timestamp quantization.
Unattested `presentation_interval` events remain a separate cadence metric.
Frame admission additionally requires a reviewed boundary-evidence digest,
one complete sequence/thread, declared clock uncertainty and observations
covering both sides of the measurement window. The digest identifies external
evidence; supplying a syntactically valid hash cannot establish a frame seam.
Comparison requires the same evidence identity in every matched group.

The process fixture launches an actual agent JVM with a separately compiled
test-only public-seam class. Four successful calls must produce three positive
intervals on the caller thread while preserving all four callee effects. This is
not a game, OpenGL driver or display-refresh experiment. Native collection still
requires the repository's bounded signed-in desktop procedure, a protected
disposable save root and matched baseline/instrumentation-off repetitions.

The fixture also checks disabled/malformed/wrong-pin startup, recording gaps
with and without an intervening presentation, a rendering-thread change, and
an actual startup JFR with owned-process identity extraction. The eight cases
passed on the host JDK and installed Java 17 runtime under Windows Job ownership.
The separate performance export/admission regression passed 41 checks. These
results establish portable preparation only; all installed-game acceptance
remains unmeasured.
