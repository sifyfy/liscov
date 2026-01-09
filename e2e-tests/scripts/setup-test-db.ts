/**
 * Test database setup script for E2E tests
 *
 * Usage:
 *   npx tsx scripts/setup-test-db.ts setup    # Insert test data
 *   npx tsx scripts/setup-test-db.ts cleanup  # Remove test data
 *   npx tsx scripts/setup-test-db.ts backup   # Backup current DB
 *   npx tsx scripts/setup-test-db.ts restore  # Restore backed up DB
 */

import { execSync } from 'child_process';
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

function ensureDbDir(): void {
  if (!fs.existsSync(DB_DIR)) {
    fs.mkdirSync(DB_DIR, { recursive: true });
    console.log(`Created directory: ${DB_DIR}`);
  }
}

function runSqlite(dbPath: string, command: string): void {
  execSync(`sqlite3 "${dbPath}" "${command}"`, { stdio: 'inherit' });
}

function readSqlFile(dbPath: string, sqlPath: string): void {
  execSync(`sqlite3 "${dbPath}" ".read '${sqlPath}'"`, { stdio: 'inherit' });
}

function backup(): void {
  ensureDbDir();
  if (fs.existsSync(DB_PATH)) {
    fs.copyFileSync(DB_PATH, DB_BACKUP_PATH);
    console.log(`Backed up: ${DB_PATH} -> ${DB_BACKUP_PATH}`);
  } else {
    console.log('No existing DB to backup');
  }
}

function restore(): void {
  if (fs.existsSync(DB_BACKUP_PATH)) {
    fs.copyFileSync(DB_BACKUP_PATH, DB_PATH);
    fs.unlinkSync(DB_BACKUP_PATH);
    console.log(`Restored: ${DB_BACKUP_PATH} -> ${DB_PATH}`);
  } else {
    console.log('No backup to restore');
  }
}

function setup(): void {
  ensureDbDir();

  // Apply schema if DB doesn't exist
  if (!fs.existsSync(DB_PATH)) {
    console.log('Creating new database with schema...');
    readSqlFile(DB_PATH, SCHEMA_PATH);
    console.log('Schema applied');
  }

  // Insert test data
  console.log('Inserting test data...');
  readSqlFile(DB_PATH, TEST_DATA_PATH);
  console.log('Test data inserted');
}

function cleanup(): void {
  if (fs.existsSync(DB_PATH)) {
    console.log('Removing test data...');
    readSqlFile(DB_PATH, CLEANUP_PATH);
    console.log('Test data removed');
  } else {
    console.log('No database to cleanup');
  }
}

// Main
const command = process.argv[2];

switch (command) {
  case 'setup':
    setup();
    break;
  case 'cleanup':
    cleanup();
    break;
  case 'backup':
    backup();
    break;
  case 'restore':
    restore();
    break;
  default:
    console.log(`
Test Database Setup Script

Usage:
  npx tsx scripts/setup-test-db.ts <command>

Commands:
  setup    - Insert test data into the database
  cleanup  - Remove test data from the database
  backup   - Backup the current database
  restore  - Restore the backed up database
`);
    process.exit(1);
}
