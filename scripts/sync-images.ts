import "dotenv/config";
import {
    S3Client,
    ListObjectsV2Command,
    PutObjectCommand,
} from "@aws-sdk/client-s3";
import sharp from "sharp";
import pLimit from "p-limit";
import { readFileSync } from "fs";
import { resolve } from "path";

interface Card {
    image_url: string;
    image_name: string;
}

const LANGUAGES = ["en", "jp"] as const;
const CONCURRENCY = 10;

const IMAGE_LIMIT = 0; // 0 = all images, >0 = limit per language

function getEnvOrThrow(name: string): string {
    const value = process.env[name];
    if (!value) {
        throw new Error(`Missing required environment variable: ${name}`);
    }
    return value;
}

function createR2Client(): { client: S3Client; bucket: string } {
    const accountId = getEnvOrThrow("R2_ACCOUNT_ID");
    const accessKeyId = getEnvOrThrow("R2_ACCESS_KEY_ID");
    const secretAccessKey = getEnvOrThrow("R2_SECRET_ACCESS_KEY");
    const bucket = getEnvOrThrow("R2_BUCKET_NAME");

    const client = new S3Client({
        region: "auto",
        endpoint: `https://${accountId}.r2.cloudflarestorage.com`,
        credentials: { accessKeyId, secretAccessKey },
    });

    return { client, bucket };
}

function loadCards(lang: string): Card[] {
    const filePath = resolve(
        import.meta.dirname,
        "..",
        "json",
        lang,
        "cards.json"
    );
    const data = readFileSync(filePath, "utf-8");
    const cards: Card[] = JSON.parse(data);

    const seen = new Set<string>();
    return cards.filter((card) => {
        if (seen.has(card.image_name)) return false;
        seen.add(card.image_name);
        return true;
    });
}

async function listExistingKeys(
    client: S3Client,
    bucket: string,
    prefix: string
): Promise<Set<string>> {
    const keys = new Set<string>();
    let continuationToken: string | undefined;

    do {
        const response = await client.send(
            new ListObjectsV2Command({
                Bucket: bucket,
                Prefix: prefix,
                ContinuationToken: continuationToken,
            })
        );

        for (const obj of response.Contents ?? []) {
            if (obj.Key) keys.add(obj.Key);
        }

        continuationToken = response.IsTruncated
            ? response.NextContinuationToken
            : undefined;
    } while (continuationToken);

    return keys;
}

async function downloadImage(url: string): Promise<Buffer> {
    const response = await fetch(url);
    if (!response.ok) {
        throw new Error(`HTTP ${response.status} downloading ${url}`);
    }
    return Buffer.from(await response.arrayBuffer());
}

async function convertToWebp(buffer: Buffer): Promise<Buffer> {
    return sharp(buffer).webp({ quality: 99 }).toBuffer();
}

async function uploadToR2(
    client: S3Client,
    bucket: string,
    key: string,
    body: Buffer
): Promise<void> {
    await client.send(
        new PutObjectCommand({
            Bucket: bucket,
            Key: key,
            Body: body,
            ContentType: "image/webp",
            CacheControl: "public, max-age=31536000, immutable",
        })
    );
}

async function main(): Promise<void> {
    console.log("=== One Piece Card Image Sync to R2 ===\n");

    const { client, bucket } = createR2Client();
    const limit = pLimit(CONCURRENCY);

    let totalProcessed = 0;
    let totalSucceeded = 0;
    let totalFailed = 0;
    const failures: string[] = [];

    for (const lang of LANGUAGES) {
        const cards = loadCards(lang);
        console.log(`[${lang}] Loaded ${cards.length} cards`);

        const prefix = `${lang}/`;
        const existingKeys = await listExistingKeys(client, bucket, prefix);
        console.log(`[${lang}] Found ${existingKeys.size} existing images in R2`);

        let missing = cards.filter(
            (card) => !existingKeys.has(`${lang}/${card.image_name}.webp`)
        );

        if (IMAGE_LIMIT > 0) missing = missing.slice(0, IMAGE_LIMIT);

        console.log(`[${lang}] ${missing.length} new images to process\n`);

        if (missing.length === 0) continue;

        let langProcessed = 0;
        const langTotal = missing.length;

        const tasks = missing.map((card) =>
            limit(async () => {
                const key = `${lang}/${card.image_name}.webp`;
                try {
                    const png = await downloadImage(card.image_url);
                    const webp = await convertToWebp(png);

                    await uploadToR2(client, bucket, key, webp);
                    totalSucceeded++;
                    console.log(`  Uploaded ${key}`);
                } catch (err) {
                    totalFailed++;
                    const msg = err instanceof Error ? err.message : String(err);
                    failures.push(`${key}: ${msg}`);
                    console.error(`  Failed ${key}: ${msg}`);
                }
                totalProcessed++;
                langProcessed++;

                if (langProcessed % 20 === 0) {
                    const remaining = langTotal - langProcessed;
                    console.log(
                        `  [${lang}] ${new Date().toISOString()} — ${remaining} images remaining`
                    );
                }
            })
        );

        await Promise.all(tasks);
        console.log();
    }

    console.log("=== Summary ===");
    console.log(`Processed: ${totalProcessed}`);
    console.log(`Succeeded: ${totalSucceeded}`);
    console.log(`Failed:    ${totalFailed}`);

    if (failures.length > 0) {
        console.log("\nFailures:");
        for (const f of failures) {
            console.log(`  - ${f}`);
        }
    }

    if (totalProcessed > 0 && totalSucceeded === 0) {
        process.exit(1);
    }
}

main().catch((err) => {
    console.error("Fatal error:", err);
    process.exit(1);
});
