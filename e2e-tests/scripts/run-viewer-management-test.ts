/**
 * Viewer Management E2E Test Runner
 *
 * Usage:
 *   npx tsx scripts/run-viewer-management-test.ts [options]
 *
 * Options:
 *   --skip-build    Skip cargo build
 *   --keep-running  Don't stop liscov after tests
 */

import { spawn, execSync, ChildProcess } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

// Paths
const PROJECT_ROOT = path.resolve(__dirname, '../..');
const E2E_ROOT = path.resolve(__dirname, '..');
// directories crate uses: ProjectDirs::from("dev", "sifyfy", "liscov").data_dir()
const DB_DIR = path.join(os.homedir(), 'AppData', 'Roaming', 'sifyfy', 'liscov', 'data');
const DB_PATH = path.join(DB_DIR, 'liscov.db');
const DB_BACKUP_PATH = path.join(DB_DIR, 'liscov.db.e2e_backup');
const SCHEMA_PATH = path.join(PROJECT_ROOT, 'src', 'database', 'schema.sql');
const TEST_DATA_PATH = path.join(E2E_ROOT, 'fixtures', 'viewer_management_test_data.sql');
const CLEANUP_PATH = path.join(E2E_ROOT, 'fixtures', 'viewer_management_cleanup.sql');
const LISCOV_EXE = path.join(PROJECT_ROOT, 'target', 'release', process.platform === 'win32' ? 'liscov.exe' : 'liscov');

const CDP_PORT = 9223;

// Parse arguments
const args = process.argv.slice(2);
const skipBuild = args.includes('--skip-build');
const keepRunning = args.includes('--keep-running');

function log(step: string, message: string, color: 'cyan' | 'yellow' | 'green' | 'red' | 'gray' = 'gray'): void {
  const colors: Record<string, string> = {
    cyan: '\x1b[36m',
    yellow: '\x1b[33m',
    green: '\x1b[32m',
    red: '\x1b[31m',
    gray: '\x1b[90m',
    reset: '\x1b[0m',
  };
  console.log(`${colors[color]}${step}${colors.reset} ${message}`);
}

function ensureDbDir(): void {
  if (!fs.existsSync(DB_DIR)) {
    fs.mkdirSync(DB_DIR, { recursive: true });
  }
}

function runSqlFile(dbPath: string, sqlPath: string): void {
  execSync(`sqlite3 "${dbPath}" ".read '${sqlPath}'"`, { stdio: 'pipe' });
}

function killLiscov(): void {
  try {
    if (process.platform === 'win32') {
      execSync('taskkill /IM liscov.exe /F', { stdio: 'pipe' });
    } else {
      execSync('pkill -f liscov', { stdio: 'pipe' });
    }
  } catch {
    // Process not running
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function waitForCDP(port: number, timeout: number = 30000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`http://localhost:${port}/json/version`);
      if (response.ok) {
        return;
      }
    } catch {
      // CDP not ready yet
    }
    await sleep(500);
  }
  throw new Error(`CDP did not become available on port ${port} within ${timeout}ms`);
}

async function main(): Promise<number> {
  let liscovProcess: ChildProcess | null = null;
  let testResult = 1;

  try {
    log('===', 'Viewer Management E2E Test ===', 'cyan');
    console.log();

    // Step 1: Stop existing liscov
    log('[1/7]', 'Stopping existing liscov processes...', 'yellow');
    killLiscov();
    await sleep(1000);
    log('  ', 'Done');

    // Step 2: Build
    if (!skipBuild) {
      log('[2/7]', 'Building release...', 'yellow');
      execSync('cargo build --release', { cwd: PROJECT_ROOT, stdio: 'inherit' });
      log('  ', 'Build complete');
    } else {
      log('[2/7]', 'Skipping build', 'gray');
    }

    // Step 3: Backup DB
    log('[3/7]', 'Backing up DB...', 'yellow');
    ensureDbDir();
    if (fs.existsSync(DB_PATH)) {
      fs.copyFileSync(DB_PATH, DB_BACKUP_PATH);
      log('  ', `Backup: ${DB_BACKUP_PATH}`);
      // Delete existing DB to start fresh with test data only
      fs.unlinkSync(DB_PATH);
      log('  ', 'Existing DB removed for clean test');
    } else {
      log('  ', 'No existing DB - will create new');
    }

    // Step 4: Prepare test DB (fresh DB with schema + test data only)
    log('[4/7]', 'Preparing test DB...', 'yellow');
    runSqlFile(DB_PATH, SCHEMA_PATH);
    log('  ', 'Schema applied');
    runSqlFile(DB_PATH, TEST_DATA_PATH);
    log('  ', 'Test data inserted');

    // Step 5: Launch liscov with CDP
    log('[5/7]', `Launching liscov (CDP: port ${CDP_PORT})...`, 'yellow');
    liscovProcess = spawn(LISCOV_EXE, [], {
      cwd: PROJECT_ROOT,
      env: {
        ...process.env,
        WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${CDP_PORT}`,
      },
      detached: false,
      stdio: 'ignore',
    });
    await sleep(3000);
    await waitForCDP(CDP_PORT, 10000);
    log('  ', `PID: ${liscovProcess.pid}`);

    // Step 6: Run Playwright tests
    log('[6/7]', 'Running Playwright tests...', 'yellow');
    try {
      execSync('npx playwright test tests/viewer-management.spec.ts --reporter=list', {
        cwd: E2E_ROOT,
        env: {
          ...process.env,
          CDP_PORT: String(CDP_PORT),
        },
        stdio: 'inherit',
      });
      testResult = 0;
    } catch (error) {
      testResult = 1;
    }

    // Step 7: Cleanup
    log('[7/7]', 'Cleaning up...', 'yellow');

    if (!keepRunning && liscovProcess && !liscovProcess.killed) {
      liscovProcess.kill();
      log('  ', 'liscov stopped');
    } else if (keepRunning) {
      log('  ', 'liscov kept running (--keep-running)');
    }

    // Remove test data
    try {
      runSqlFile(DB_PATH, CLEANUP_PATH);
      log('  ', 'Test data deleted');
    } catch {
      log('  ', 'Test data deletion skipped');
    }

    // Restore DB
    if (fs.existsSync(DB_BACKUP_PATH)) {
      fs.copyFileSync(DB_BACKUP_PATH, DB_PATH);
      fs.unlinkSync(DB_BACKUP_PATH);
      log('  ', 'DB restored');
    }

  } catch (error) {
    console.error('Error:', error);
    testResult = 1;

    // Cleanup on error
    if (liscovProcess && !liscovProcess.killed) {
      liscovProcess.kill();
    }
    if (fs.existsSync(DB_BACKUP_PATH)) {
      fs.copyFileSync(DB_BACKUP_PATH, DB_PATH);
      fs.unlinkSync(DB_BACKUP_PATH);
    }
  }

  // Show result
  console.log();
  if (testResult === 0) {
    log('===', 'TEST PASSED ===', 'green');
  } else {
    log('===', 'TEST FAILED ===', 'red');
  }

  return testResult;
}

main().then(code => process.exit(code));
