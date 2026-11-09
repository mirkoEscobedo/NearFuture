// @ts-check
// Vendor7.1 native-width window captures start at the client DC, not outer bounds.
import {isDeepStrictEqual as same} from 'node:util';
import {closed,requireThat} from './campaign-proof.mjs';

/** Pure conversion only: callers must obtain the real read-only owned geometry
 * after image inspection and retain fresh focus checks and action authority.
 * @param {{geometry:any,window:any,owned:any,dimensions:any,point:any,nowMs:number}} input */
export function clientImagePoint({geometry:g,window:w,owned:o,dimensions:d,point:p,nowMs}) {
 const need=(/** @type {any} */ ok)=>requireThat(ok,'measured client image coordinate required');
 const integer=Number.isSafeInteger;
 need(closed(g,['schemaVersion','scope','windowId','pid','creationTime','controllerPid','observedAtUtc',
  'outerBounds','clientOriginScreen','clientSize','offsetFromOuter','mcpOuterBoundsMatched','inputGranted','originMeaning']));
 need(g.schemaVersion===1&&g.scope==='READ_ONLY_OWNED_HWND_CLIENT_GEOMETRY'&&g.mcpOuterBoundsMatched===true&&g.inputGranted===false&&
  integer(g.windowId)&&g.windowId>0&&integer(g.pid)&&g.pid>0&&integer(g.controllerPid)&&g.controllerPid>0&&
  w?.windowId===g.windowId&&w.pid===g.pid&&w.isFocused===true&&o?.pid===g.pid&&o.controllerPid===g.controllerPid&&
  o.inSupervisorJob===true&&typeof g.creationTime==='string'&&Number.isFinite(Date.parse(g.creationTime))&&o.creationTime===g.creationTime);
 need(closed(g.outerBounds,['x','y','width','height'])&&Object.values(g.outerBounds).every(integer)&&
  g.outerBounds.width>0&&g.outerBounds.height>0&&same(g.outerBounds,w.bounds)&&
  closed(d,['width','height'])&&d.width===w.bounds.width&&d.height===w.bounds.height);
 const origin=g.clientOriginScreen,size=g.clientSize,offset=g.offsetFromOuter;
 need(closed(origin,['x','y'])&&Object.values(origin).every(integer)&&closed(size,['width','height'])&&
  Object.values(size).every(integer)&&size.width>0&&size.height>0&&closed(offset,['x','y'])&&Object.values(offset).every(integer)&&
  offset.x===origin.x-w.bounds.x&&offset.y===origin.y-w.bounds.y&&offset.x>=0&&offset.y>=0&&
  offset.x+size.width<=d.width&&offset.y+size.height<=d.height);
 const observed=typeof g.observedAtUtc==='string'?Date.parse(g.observedAtUtc):NaN;
 need(integer(nowMs)&&Number.isFinite(observed)&&observed<=nowMs&&nowMs-observed<=5000&&
  closed(p,['x','y'])&&integer(p.x)&&integer(p.y)&&p.x>=0&&p.y>=0&&p.x<size.width&&p.y<size.height);
 const screen=[origin.x+p.x,origin.y+p.y];need(screen.every(integer));return screen;
}
