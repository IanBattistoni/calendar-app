import assert from 'node:assert/strict';
import { test } from 'node:test';
import { escapeHtml } from '../src/events.ts';
test('event text is escaped before rendering in markup and attributes', () => {
  assert.equal(escapeHtml('<img src=x onerror="alert(1)"> & \'test\''), '&lt;img src=x onerror=&quot;alert(1)&quot;&gt; &amp; &#39;test&#39;');
});
