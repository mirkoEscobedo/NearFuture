'use strict';
const {uint} = require('./fields.cjs');
function addScheduling(c,row) {
  const model=(name,expectation,input)=>{
    row('model',name,'local',expectation,Buffer.from(JSON.stringify(input),'utf8'),{wire_record:false});
  };
  model('outbound-rate-burst-exhaustion','RATE_LIMIT_NO_SEND',{rate:8,burst:8,elapsed_ns:'0',attempts:9,maximum_admitted:8});
  model('inbound-rate-refusal','RATE_LIMIT_NO_ACK',{rate:8,burst:8,elapsed_ns:'0',attempts:64,maximum_admitted:8});
  model('bucket-replenish-with-cap','AT_MOST_BURST',{rate:8,burst:8,elapsed_ns:'1000000000000',initial_tokens:0,maximum_tokens:8});
  model('backwards-owner-observation','CLOSE_LOCAL_LANE',{previous_ns:'2000000000',current_ns:'1000000000'});
  model('subscribe-proof-at-five-second-deadline','CLOSE_LOCAL_LANE',{issued_ns:'0',now_ns:'5000000000',deadline_ns:'5000000000'});
  model('subscription-local-start-not-extended','CLOSE_LOCAL_LANE',{lifetime_seconds:1,client_start_ns:'0',response_ns:'1000000000'});
  model('notice-ack-at-five-second-deadline','CLOSE_LOCAL_LANE',{sent_ns:'0',now_ns:'5000000000',deadline_ns:'5000000000'});
  model('sequence-overflow','CLOSE_LOCAL_LANE',{current_sequence:'18446744073709551615',attempted_next:'18446744073709551616'});
  model('repeated-terminal-observation','ONE_DIRTY_HINT',{projection_before:'Pending',projection_after:'Committed7',repeated_equal_callbacks:64,maximum_new_hints:1});
  model('dirty-during-query','ONE_FOLLOW_UP',{steps:['dirty1','queryBegin1','dirty2','queryFinish1'],remaining_dirty_generation:2,maximum_followup_queries:1});
  model('one-awaiting-notice-plus-dirty-marker','BOUNDED_TWO_SLOTS',{awaited:1,marker:1,maximum_awaited:1,maximum_marker:1});
  model('round-robin-ready-notify','CONTROL_BULK_PROGRESS',{ready_lanes:[1,2,3],rotating_rounds:3,events_per_lane_per_round:1,deadline_sweep_each_round:true});
  row('capacity','three-lane-global-challenge-cap','local','AT_MOST_THREE_CHALLENGES',uint(8,3),{per_lane:1,lanes:3});
}
module.exports={addScheduling};
