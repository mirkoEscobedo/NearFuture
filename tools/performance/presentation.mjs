// @ts-check
/** A certificate identifies reviewed evidence; hash syntax never attests its contents.
 * @param {import('./contract.mjs').RunMetadata} run
 * @param {import('./contract.mjs').TraceEvent[]} events
 * @param {number} start @param {number} end
 * @returns {import('./contract.mjs').TraceEvent[]} */
export function admittedPresentationFrames(run,events,start,end) {
  const intervals=events.filter(event=>event.kind==='presentation_interval').sort((a,b)=>Number(a.frameId)-Number(b.frameId));
  const certificate=run.frameBoundaryEvidenceSha256;
  if (!intervals.length) {
    if (certificate) throw new Error('MISSING_PRESENTATION: the attested recording has no presentation observations.');
    return [];
  }
  const tolerance=2*run.alignmentUncertaintyUs+2;
  if (!Number.isSafeInteger(tolerance)) throw new Error('INVALID_RUN: presentation uncertainty exceeds safe precision.');
  for (let index=0;index<intervals.length;index++) {
    const current=intervals[index], previous=intervals[index-1];
    if ((current.clockUncertaintyUs??0)>run.alignmentUncertaintyUs)
      throw new Error('PRESENTATION_CLOCK_UNCERTAINTY: the run must cover every observed marker calibration.');
    if (current.frameId!==index+1 || current.threadId!==intervals[0].threadId)
      throw new Error('PRESENTATION_LOSS: require one complete, sequential, single-thread recording.');
    if (!Number.isSafeInteger(current.atUs+(current.durationUs??0)))
      throw new Error('INVALID_EVENT: presentation endpoint exceeds safe precision.');
    if (previous && Math.abs(current.atUs-previous.atUs-(previous.durationUs??0))>tolerance)
      throw new Error('PRESENTATION_CLOCK_GAP: endpoints exceed the declared alignment uncertainty.');
  }
  if (!certificate) return [];
  if (events.some(event=>event.kind==='frame'))
    throw new Error('AMBIGUOUS_FRAME_SOURCE: select the attested presentation source or an explicit frame source.');
  const last=intervals[intervals.length-1];
  if (intervals[0].atUs>start || last.atUs+(last.durationUs??0)<end)
    throw new Error('INCOMPLETE_PRESENTATION_WINDOW: retain observations before and after the measurement window.');
  return intervals;
}
