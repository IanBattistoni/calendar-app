import assert from 'node:assert/strict';
import { test } from 'node:test';
import { remainingTime, upcomingRows } from '../src/widget-data.ts';
test('countdown handles due, minute, hour and day boundaries', () => {
 const now=100000;
 assert.equal(remainingTime(now-1,now),'Ahora');
 assert.equal(remainingTime(now+60000,now),'En 1 min');
 assert.equal(remainingTime(now+90*60000,now),'En 1 h 30 min');
 assert.equal(remainingTime(now+1440*60000,now),'En 1 día');
});
test('upcoming list is bounded and escapes event titles', () => {
 const item={event:{id:'a',title:'<script>bad</script>',time:null},date:'2026-10-21',eventAt:Date.UTC(2026,9,21,12),remindAt:null,expiresAt:Date.UTC(2026,9,22)};
 const html=upcomingRows([item,item,item,item],3);
 assert.equal((html.match(/class="upcoming-row"/g)||[]).length,3);
 assert.ok(!html.includes('<script>'));
 assert.ok(html.includes('&lt;script&gt;'));
});


import { miniEvent } from '../src/widget-data.ts';
import { expansionState } from '../src/widget-state.ts';

test('mini card shows event time rather than reminder deadline and escapes titles', () => {
 const item={event:{id:'<x>',title:'<img src=x>',time:'09:00'},date:'2026-10-21',eventAt:1792584000000,remindAt:1792583100000};
 const html=miniEvent(item,true);
 assert.ok(html.includes('data-countdown="1792584000000"'));
 assert.ok(html.includes('09:00'));
 assert.ok(!html.includes('<img'));
});

test('rapid hover changes converge to collapsed native and content state', async () => {
 const calls=[], paints=[], pending=[];
 const state=expansionState(next=>{calls.push(next);return new Promise(resolve=>pending.push(resolve));},next=>paints.push(next),assert.fail);
 state.request(true);state.request(false);state.request(true);state.request(false);
 assert.deepEqual(calls,[true]);assert.deepEqual(paints,[true]);
 pending.shift()();await new Promise(setImmediate);
 assert.deepEqual(calls,[true,false]);assert.deepEqual(paints,[true,false]);
 pending.shift()();await new Promise(setImmediate);
 assert.equal(state.desired(),false);
});

test('dragging pauses resizing and collapses when the native drag ends', async () => {
 const calls=[];
 const state=expansionState(async next=>{calls.push(next);},()=>{},assert.fail);
 state.pause(true);state.request(true);assert.deepEqual(calls,[]);
 state.pause(false);await new Promise(setImmediate);assert.deepEqual(calls,[]);
 state.request(true);await new Promise(setImmediate);
 state.pause(true);state.request(false);assert.deepEqual(calls,[true]);
 state.pause(false);await new Promise(setImmediate);assert.deepEqual(calls,[true,false]);
});

test('native resize failure restores collapsed content and attempts native rollback', async () => {
 const calls=[],paints=[],errors=[];
 const state=expansionState(async next=>{calls.push(next);if(next)throw Error('resize');},next=>paints.push(next),error=>errors.push(error));
 state.request(true);await new Promise(setImmediate);
 assert.deepEqual(calls,[true,false]);assert.deepEqual(paints,[true,false]);assert.equal(errors.length,1);
});

test('content remeasurement is serialized even when the hover state stays unchanged', async () => {
 const calls=[],pending=[];
 const state=expansionState(next=>{calls.push(next);return new Promise(resolve=>pending.push(resolve));},()=>{},assert.fail);
 state.request(false,true);state.request(false,true);
 assert.deepEqual(calls,[false]);
 pending.shift()();await new Promise(setImmediate);
 assert.deepEqual(calls,[false,false]);
 pending.shift()();await new Promise(setImmediate);
 assert.deepEqual(calls,[false,false]);
});
