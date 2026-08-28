// Copyright (c) Meta Platforms, Inc. and affiliates.

import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

import {describe, expect, it} from 'vitest';

const ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  '../../..',
);
const WORKFLOWS = path.join(ROOT, '.github', 'workflows');

function workflow(name) {
  return fs.readFileSync(path.join(WORKFLOWS, name), 'utf8');
}

describe('visual acceptance workflow concurrency', () => {
  it('locks PR report resolution before any mutation', () => {
    const value = workflow('pr-comment.yml');
    const [header, jobs] = value.split('\njobs:\n');

    expect(header).toContain(
      'group: visual-acceptance-head-${{ github.event.workflow_run.head_repository.id }}-${{ github.event.workflow_run.head_branch }}',
    );
    expect(header).toContain(
      "cancel-in-progress: ${{ github.event.action == 'requested' || github.event.action == 'in_progress' }}",
    );
    expect(jobs).not.toContain('visual-acceptance-pr-');
    expect(jobs).not.toContain('    concurrency:');
  });

  it('keeps status initialization out of pull-request checks', () => {
    const acceptance = workflow('visual-acceptance.yml');
    const publisher = workflow('pr-comment.yml');

    expect(acceptance).not.toContain('pull_request_target:');
    expect(acceptance).not.toContain('  initialize:');
    expect(publisher).toContain('types: [requested, in_progress, completed]');
    expect(publisher).toContain(
      'description: `CI run ${run.id}/${run.run_attempt} is producing fresh visual evidence.`',
    );
  });

  it('keeps comment authorization read-only until the shared lock is held', () => {
    const value = workflow('visual-acceptance.yml');
    const authorize = value.slice(
      value.indexOf('  authorize:'),
      value.indexOf('  accept:'),
    );
    const authorizeCheckout = authorize.slice(
      authorize.indexOf('      - name: Checkout trusted default-branch code'),
      authorize.indexOf('      - name: Authorize and resolve the decision'),
    );
    const accept = value.slice(value.indexOf('  accept:'));

    expect(authorize).not.toContain(': write');
    expect(authorize).toContain('actions/checkout@v7');
    expect(authorizeCheckout).not.toContain('ref:');
    expect(authorize).toContain('visualAcceptanceIdentity(response.data)');
    expect(authorize).toContain(
      'isVisualAcceptanceEndpointMaintainer(identity)',
    );
    expect(authorize).toContain(
      "core.setOutput('effective_permission', identity.effectivePermission)",
    );
    expect(authorize).toContain(
      "core.setOutput('role_name', identity.roleName ?? '')",
    );
    expect(authorize).not.toContain('author_association');
    expect(authorize).not.toContain('issues.createComment');
    expect(authorize).not.toContain('createCommitStatus');
    expect(accept).toContain('needs: authorize');
    expect(accept).toContain(
      'EFFECTIVE_PERMISSION: ${{ needs.authorize.outputs.effective_permission }}',
    );
    expect(accept).toContain(
      'ROLE_NAME: ${{ needs.authorize.outputs.role_name }}',
    );
    expect(accept).toContain('--effective-permission "$EFFECTIVE_PERMISSION"');
    expect(accept).toContain('--role-name "$ROLE_NAME"');
    expect(accept).toContain(
      'group: visual-acceptance-head-${{ needs.authorize.outputs.head_repo_id }}-${{ needs.authorize.outputs.head_ref }}',
    );
    expect(accept).toContain('cancel-in-progress: false');
    expect(accept).toContain('pull-requests: write');
    expect(accept).not.toContain('issues: write');
  });

  it('invalidates the advisory label only in the trusted workflow_run publisher', () => {
    const value = workflow('pr-comment.yml');
    const invalidate = value.slice(
      value.indexOf('  invalidate:'),
      value.indexOf('  comment:'),
    );

    expect(invalidate).toContain('pull-requests: write');
    expect(invalidate).toContain('statuses: write');
    expect(invalidate).toContain('createCommitStatus');
    expect(invalidate).toContain('issues.removeLabel');
    expect(invalidate.indexOf('createCommitStatus')).toBeLessThan(
      invalidate.indexOf('issues.removeLabel'),
    );
    const publisher = value.slice(value.indexOf('  comment:'));
    expect(publisher).toContain("if (state.reason === 'accepted')");
    expect(publisher).toContain('issues.addLabels');
    expect(publisher).toContain('issues.removeLabel');
  });

  it('passes source CI identity without overriding reserved GitHub variables', () => {
    const value = workflow('pr-comment.yml');
    const capture = value.slice(
      value.indexOf('      - name: Capture the trusted stable visual scope'),
      value.indexOf('      # The Storybook bundle is untrusted.'),
    );

    expect(capture).toContain(
      'ASTRYX_VISUAL_SHA: ${{ steps.identity.outputs.head_sha }}',
    );
    expect(capture).toContain(
      'ASTRYX_VISUAL_RUN_ID: ${{ steps.identity.outputs.run_id }}',
    );
    expect(capture).toContain(
      'ASTRYX_VISUAL_RUN_ATTEMPT: ${{ steps.identity.outputs.run_attempt }}',
    );
    expect(capture).not.toContain('GITHUB_SHA:');
    expect(capture).not.toContain('GITHUB_RUN_ID:');
    expect(capture).not.toContain('GITHUB_RUN_ATTEMPT:');
  });

  it('defers broad trusted scopes before downloading or capturing Storybook', () => {
    const value = workflow('pr-comment.yml');
    const download = value.slice(
      value.indexOf(
        '      - name: Download Storybook for trusted visual capture',
      ),
      value.indexOf('      - name: Cross-check artifact identity'),
    );
    const capture = value.slice(
      value.indexOf('      - name: Capture the trusted stable visual scope'),
      value.indexOf('      - name: Derive trusted broad visual deferral'),
    );
    const defer = value.slice(
      value.indexOf('      - name: Derive trusted broad visual deferral'),
      value.indexOf('      # The Storybook bundle is untrusted.'),
    );
    const derive = value.slice(
      value.indexOf('      - name: Derive trusted visual evidence and report'),
      value.indexOf('      - name: Resolve trusted visual evidence path'),
    );

    const resolve = value.slice(
      value.indexOf('      - name: Resolve trusted visual evidence path'),
      value.indexOf('      - name: Publish immutable visual evidence'),
    );

    expect(download).toContain("steps.scope.outputs.exact == 'true'");
    expect(capture).toContain("steps.scope.outputs.exact == 'true'");
    expect(capture).toContain('trusted-plan');
    expect(capture).toContain('"$SHOTS" -gt 24');
    expect(capture).toContain('"$SHOTS" -gt 40');
    expect(capture).toContain('--plan-file trusted-plan.json');
    expect(capture.indexOf('trusted-plan')).toBeLessThan(
      capture.indexOf('"$SHOTS" -gt 40'),
    );
    expect(capture.indexOf('"$SHOTS" -gt 40')).toBeLessThan(
      capture.indexOf('npx playwright install chromium'),
    );
    expect(defer).toContain("steps.scope.outputs.broad == 'true'");
    expect(defer).toContain("steps.scope.outputs.exact != 'true'");
    expect(defer).toContain('visual-acceptance.mjs trusted-defer');
    expect(defer).toContain('--run-attempt "$RUN_ATTEMPT"');
    expect(defer).not.toContain('playwright');
    expect(defer).not.toContain('gate.mjs capture');
    expect(derive).toContain("steps.scope.outputs.exact == 'true'");
    expect(derive).toContain("steps.plan.outputs.deferred != 'true'");
    expect(value).toContain(
      "core.setOutput('exact', String(scope.exactStableVisual))",
    );
    expect(resolve).toContain('test -f trusted-visual/evidence.json');
    expect(resolve).toContain(
      'path=pr/${PR_NUMBER}/visual/${HEAD_SHA}/${RUN_ID}/${RUN_ATTEMPT}',
    );
  });

  it('uses affected dependency tracing for PR a11y scope instead of broad sweeps', () => {
    const value = workflow('ci.yml');
    const check = value.slice(
      value.indexOf('  check-components:'),
      value.indexOf('  # Run tests'),
    );
    const prA11y = value.slice(
      value.indexOf('  pr-a11y:'),
      value.indexOf('  # The layer order'),
    );

    expect(check).toContain(
      'a11y_deferred: ${{ steps.files.outputs.a11y_deferred || steps.affected.outputs.a11y_deferred }}',
    );
    expect(check).toContain('uses: ./.github/actions/affected-scope');
    expect(check).toContain(
      'changed-files: ${{ steps.files.outputs.changed_files }}',
    );
    expect(check).toContain('visual-scope.mjs --github-output');
    expect(check).toContain('has_components=false');
    expect(check).toContain('has_stable_visual=false');
    expect(check).toContain('stable_visual_deferred=true');
    expect(check).toContain('a11y_deferred=true');
    expect(check).toContain(
      'deferring PR-scoped a11y and visual review to protected main',
    );
    expect(check).not.toContain(
      'assuming components changed so pr-a11y still runs',
    );
    expect(prA11y).toContain(
      "needs.check-components.outputs.has_components == 'true'",
    );
  });

  it('routes every PR-scope consumer through the shared affected-scope action', () => {
    const ci = workflow('ci.yml');
    const action = fs.readFileSync(
      path.join(ROOT, '.github/actions/affected-scope/action.yml'),
      'utf8',
    );
    const check = ci.slice(
      ci.indexOf('  check-components:'),
      ci.indexOf('  # Run tests'),
    );
    const prA11y = ci.slice(
      ci.indexOf('  pr-a11y:'),
      ci.indexOf('  # The layer order'),
    );
    const prVisual = ci.slice(
      ci.indexOf('  pr-visual:'),
      ci.indexOf('  # RTL semantic audit'),
    );
    const prRtl = ci.slice(
      ci.indexOf('  pr-rtl:'),
      ci.indexOf('      - name: Write scorecard summary'),
    );

    for (const block of [check, prA11y, prVisual, prRtl]) {
      expect(block).toContain('uses: ./.github/actions/affected-scope');
      expect(block).not.toContain('node .github/scripts/lib/affected-scope.js');
      expect(block).not.toContain('.newComponents + .modifiedComponents');
      expect(block).not.toContain("jq -r '[.affectedScope.components");
    }
    expect(check).toContain(
      'changed-files: ${{ steps.files.outputs.changed_files }}',
    );
    for (const block of [prA11y, prVisual, prRtl]) {
      expect(block).toContain('analysis-file: analysis.json');
      expect(block).toContain('steps.affected.outputs.affected_');
    }
    expect(action).toContain('inputs:');
    expect(action).toContain('changed-files:');
    expect(action).toContain('analysis-file:');
    expect(action).toContain('outputs:');
    expect(action).toContain('affected_components:');
    expect(action).toContain('affected_core_components:');
    expect(action).toContain('rtl_deferred:');
    expect(action).toContain('using: node20');
    expect(action).toContain('main: index.mjs');
    expect(action).not.toContain('run:');
  });

  it('uses the documented collaborator permission response shape', () => {
    const value = workflow('visual-acceptance.yml');
    const authorize = value.slice(
      value.indexOf('  authorize:'),
      value.indexOf('  accept:'),
    );

    expect(authorize).toContain('visualAcceptanceIdentity(response.data)');
    expect(authorize).not.toContain('response.data.user.permission');
  });

  it('installs the dependencies used by the acceptance archive script', () => {
    const value = workflow('visual-acceptance.yml');
    const accept = value.slice(value.indexOf('  accept:'));

    expect(accept.indexOf('uses: ./.github/actions/setup')).toBeGreaterThan(-1);
    expect(accept.indexOf('uses: ./.github/actions/setup')).toBeLessThan(
      accept.indexOf(
        'node .github/scripts/visual-gate/visual-acceptance.mjs accept',
      ),
    );
  });

  it('serializes normal, recovery, and manual publication without Actions cancellation', () => {
    const value = workflow('visual-acceptance-promote.yml');
    const manual = workflow('visual-baseline.yml');
    const helper =
      'node .github/scripts/visual-gate/lib/baseline-publication-lock.mjs';

    expect(value).toContain('workflow_dispatch:');
    expect(value).toContain("context.ref !== 'refs/heads/main'");
    expect(value).toContain('Checkout trusted current main');
    expect(value).toContain('Checkout the resolved merged result');
    expect(value).toContain('allow-unsafe-pr-checkout: true');
    expect(value.indexOf('compareCommitsWithBasehead')).toBeLessThan(
      value.indexOf('allow-unsafe-pr-checkout: true'),
    );
    expect(value).toContain('ref: main');
    for (const command of ['enqueue', 'wait', 'release']) {
      expect(value.match(new RegExp(`${helper} ${command}`, 'g'))).toHaveLength(
        1,
      );
      expect(
        manual.match(new RegExp(`${helper} ${command}`, 'g')),
      ).toHaveLength(1);
    }
    expect(value).not.toContain('group: visual-baseline');
    expect(manual).not.toContain('group: visual-baseline');
    expect(value.indexOf('Recapture exactly the accepted shots')).toBeLessThan(
      value.indexOf('Wait for the baseline publication turn'),
    );
    expect(
      value.indexOf('Wait for the baseline publication turn'),
    ).toBeLessThan(value.indexOf('Verify and promote the baseline'));
    expect(value).toContain('--expected-record-rel "$RECORD_REL"');
    expect(workflow('pr-comment.yml')).toContain(
      'Post-merge promotion does not join this cancellation group',
    );
    expect(
      manual.indexOf('Wait for the baseline publication turn'),
    ).toBeLessThan(manual.indexOf('Fetch the current baseline'));
    expect(workflow('release-gate.yml')).toContain(
      "grep -Ev '/(baseline|latest|publication-queue)/$'",
    );
    expect(value).toContain(
      "context.eventName === 'workflow_dispatch' ? 'true' : 'false'",
    );
  });

  it('projects every known validation or publication failure from an always-running job', () => {
    const value = workflow('visual-acceptance-promote.yml');
    const status = value.slice(value.indexOf('  project-status:'));

    expect(status).toContain('if: always()');
    expect(status).toContain('statuses: write');
    expect(status).toContain('needs: [resolve, promote]');
    expect(status).toContain('needs.resolve.outputs.failure_description');
    expect(status).toContain('needs.promote.outputs.failure_description');
    expect(status).toContain('promotionStatusProjection');
    expect(status).toContain('target_url: process.env.TARGET_URL');
    expect(status).toContain("if (projection.state === 'failure')");
    expect(status).toContain('core.setFailed(projection.description)');
    expect(value.match(/core\.setOutput\('head_sha'/g)).toHaveLength(1);
    expect(value.indexOf('compareCommitsWithBasehead')).toBeLessThan(
      value.indexOf("core.setOutput('head_sha'"),
    );
    expect(value).toContain('core.setFailed(failure.description)');
    const beforeStatus = value.slice(0, value.indexOf('  project-status:'));
    expect(beforeStatus).toContain("state: 'pending'");
    expect(beforeStatus).not.toContain("state: 'success'");
    expect(beforeStatus).not.toContain("state: 'failure'");
    const promoteJob = value.slice(
      value.indexOf('  promote:'),
      value.indexOf('  project-status:'),
    );
    expect(promoteJob).toContain(
      "needs.resolve.outputs.acceptance_found == 'true'",
    );
    expect(
      promoteJob.slice(0, promoteJob.indexOf('    runs-on:')),
    ).not.toContain('mutation_deferred');
    expect(promoteJob).toContain('Confirm trusted active-retry deferral');
    expect(promoteJob).toContain('deferred=true');
    expect(promoteJob).toContain("if: steps.defer.outputs.deferred != 'true'");
    expect(value).toContain(
      "mutation_deferred: ${{ steps.defer.outputs.deferred == 'true' || steps.acceptance.outputs.deferred == 'true' || steps.promote.outputs.deferred == 'true' }}",
    );
    expect(status).toContain(
      "MUTATION_DEFERRED: ${{ needs.promote.outputs.mutation_deferred == 'true' }}",
    );
    expect(status).not.toContain('needs.resolve.outputs.mutation_deferred');
    const projectionCall = status.slice(
      status.indexOf('const projection = promotionStatusProjection({'),
    );
    expect(projectionCall).toContain(
      'promotionResult: recoveryOperationResult({',
    );
    expect(projectionCall).toContain(
      'mutationDeferred: process.env.MUTATION_DEFERRED',
    );
    expect(projectionCall).toContain(
      'failureDescription: process.env.FAILURE_DESCRIPTION',
    );
  });

  it('marks success only after publication, gate dispatch, and lock release finish', () => {
    const value = workflow('visual-acceptance-promote.yml');
    const gate = value.slice(
      value.indexOf('      - name: Run a fresh release gate'),
      value.indexOf('      - name: Release the baseline publication turn'),
    );
    const release = value.slice(
      value.indexOf('      - name: Release the baseline publication turn'),
      value.indexOf('      - name: Mark trusted recovery complete'),
    );
    const complete = value.slice(
      value.indexOf('      - name: Mark trusted recovery complete'),
      value.indexOf('  project-status:'),
    );

    expect(value).toContain('publication_confirmed=true');
    expect(gate).toContain(
      "steps.promote.outputs.publication_confirmed == 'true'",
    );
    expect(gate).toContain('fresh release gate dispatch failed');
    expect(release).toContain('if: always()');
    expect(release).toContain('lock release failed');
    expect(complete).toContain("steps.gate.outcome == 'success'");
    expect(complete).toContain("steps.release.outcome == 'success'");
    expect(complete).toContain('recovery_complete=true');
    expect(value).toContain(
      'RECOVERY_COMPLETE: ${{ needs.promote.outputs.recovery_complete }}',
    );
    expect(value).toContain('recoveryOperationResult({');
  });
});
