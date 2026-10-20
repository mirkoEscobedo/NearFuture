// @ts-check
/** @typedef {{knownTotal:()=>number,included:()=>number,omitted:()=>number,completeWithinScope:()=>boolean,origin:()=>string,atomicity:()=>string}} Coverage */
/** @typedef {{ordinal:()=>number,publicPackage:()=>string,version:()=>string,stateSchema:()=>null,provenance:()=>string}} Module */
/** @typedef {{ordinal:()=>number,moduleOrdinal:()=>number,role:()=>string,byteLength:()=>number,sha256:()=>string,classification:()=>string}} Artifact */
/** @typedef {{kind:()=>string,value:()=>number|boolean|string}} Flag */
/** @typedef {{origin:()=>string,os:()=>string,arch:()=>string,runtimeFamily:()=>string,runtimeVersion:()=>string|null,jvmFlags:()=>ReadonlyArray<Flag>}} Declared */
/** @typedef {{schema:()=>string,scope:()=>string,coverage:()=>Coverage,modules:()=>ReadonlyArray<Module>,artifacts:()=>ReadonlyArray<Artifact>,declaredRuntime:()=>Declared|null,liveRuntime:()=>string,embeddedSourceRevision:()=>null,certification:()=>string,limitations:()=>ReadonlyArray<string>}} Observation */
/** Test-only direct getter projection. No production encoder or expected-value derivation.
 * @param {Observation} observation */
export function getterProjection(observation) {
  const coverage = observation.coverage();
  const declared = observation.declaredRuntime();
  return JSON.stringify({
    schema: observation.schema(),
    scope: observation.scope(),
    coverage: {
      known_total: coverage.knownTotal(), included: coverage.included(),
      omitted: coverage.omitted(), complete_within_scope: coverage.completeWithinScope(),
      origin: coverage.origin(), atomicity: coverage.atomicity(),
    },
    modules: observation.modules().map(module => ({
      alias: 'module' + module.ordinal(), public_package: module.publicPackage(),
      version: module.version(), state_schema: module.stateSchema(),
      provenance: module.provenance(),
    })),
    artifacts: observation.artifacts().map(artifact => ({
      alias: 'artifact' + artifact.ordinal(),
      module: 'module' + artifact.moduleOrdinal(), role: artifact.role(),
      byte_length: artifact.byteLength(), sha256: artifact.sha256(),
      classification: artifact.classification(),
    })),
    declared_runtime: declared ? {
      origin: declared.origin(), os: declared.os(), arch: declared.arch(),
      runtime_family: declared.runtimeFamily(), runtime_version: declared.runtimeVersion(),
      jvm_flags: declared.jvmFlags().map(flag => ({ kind: flag.kind(), value: flag.value() })),
    } : null,
    live_runtime: observation.liveRuntime(),
    embedded_source_revision: observation.embeddedSourceRevision(),
    certification: observation.certification(),
    limitations: observation.limitations(),
  });
}
/** @param {Observation} observation */
export function immutableValues(observation) {
  const declared = observation.declaredRuntime();
  return [observation, observation.coverage(), observation.modules(),
    ...observation.modules(), observation.artifacts(), ...observation.artifacts(),
    observation.limitations(), ...(declared ? [declared, declared.jvmFlags(),
      ...declared.jvmFlags()] : [])];
}
