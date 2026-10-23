// @ts-check
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {seedFixture,json,write,filePin} from './seed-test-support.mjs';
import {recordingFixture,defaultJfc} from './recording-profile-test-support.mjs';

/** Existing generated public protection fixture; no game/JFR/vault executable.
 * Real Read-NativeFixtureBinding and protection-check.ps1 admit-launch are used.
 * @param {1|2} [version] */
export function recordingAdmissionFixture(version=2){
 const f=seedFixture();
 if(version===1){f.launch('neither');return {...f,profileDigest:undefined,refreshAuthority(){},refreshGeneratedPins(){}};}
 const r=recordingFixture();
 try{
  const recipe=r.recipe('$jfc.OuterXml');assert.equal(recipe.status,0,recipe.stderr);
  for(const root of [f.original,f.installation]){
   write(resolve(root,'jre/lib/jfr/default.jfc'),defaultJfc);
   write(resolve(root,'jre/release'),'JAVA_VERSION="test-only"');
  }
  const profilePath=resolve(f.target,'recording-profile.private.json');
  const profileDigest=json(profilePath,r.profile).sha256;
  const binding=JSON.parse(readFileSync(f.bindingPath,'utf8'));
  binding.schemaVersion=2;binding.recordingProfileSha256=profileDigest;
  Object.assign(f.bindingPin,json(f.bindingPath,binding));
  write(resolve(f.target,'capture/active.jfc'),recipe.stdout.trim());
  write(resolve(f.installation,'vmparams'),'-Xmx1g -XX:StartFlightRecording=name=NF99,settings='+f.target+'/capture/active.jfc,duration=420s,maxsize=64m,dumponexit=true,filename='+f.target+'/capture/current.jfr -classpath synthetic');
  const frozen=JSON.parse(readFileSync(f.frozenPath,'utf8'));frozen.recordingProfileSha256=profileDigest;
  frozen.jfcSha256=filePin(resolve(f.target,'capture/active.jfc')).sha256;
  frozen.vmparamsSha256=filePin(resolve(f.installation,'vmparams')).sha256;json(f.frozenPath,frozen);
  f.launch('neither');
  /** Re-pin the outer documents so refusal must arise from the tested recipe/authority. */
  function refreshAuthority(){
   const planPath=resolve(f.admission,'plan.json'),leasePath=resolve(f.admission,'native-lane-lease.private.json');
   const plan=JSON.parse(readFileSync(planPath,'utf8')),lease=JSON.parse(readFileSync(leasePath,'utf8'));
   plan.recordingProfileSha256=profileDigest;lease.recordingProfileSha256=profileDigest;
   lease.planSha256=json(planPath,plan).sha256;lease.fixtureReceiptSha256=filePin(f.frozenPath).sha256;json(leasePath,lease);
  }
  function refreshGeneratedPins(){
   const receipt=JSON.parse(readFileSync(f.frozenPath,'utf8'));
   receipt.jfcSha256=filePin(resolve(f.target,'capture/active.jfc')).sha256;
   receipt.vmparamsSha256=filePin(resolve(f.installation,'vmparams')).sha256;json(f.frozenPath,receipt);refreshAuthority();
  }
  refreshAuthority();
  return {...f,profileDigest,refreshAuthority,refreshGeneratedPins};
 }catch(error){f.close();throw error;}finally{r.close();}
}

