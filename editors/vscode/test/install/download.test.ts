// What the extension downloads, and everything it refuses to hand back.
//
// The verification is not a step the caller can forget: `downloadVerified` either returns bytes the
// release's own `SHA256SUMS` vouches for or it throws, and there is no state in between
// (`rule:ide/the-extension-guides-an-install-and-never-bundles-one`, ADR 0155 § 5). These cases are
// the ones that would notice if that stopped being true — a tampered archive, a release that lists
// no sum for it, and a fetch that never arrives.

import * as assert from "node:assert/strict";

import {
  assetUrl, archiveName, downloadVerified, expectedHash, RELEASE_LIST, releaseFor, SUMS,
} from "../../src/install";
import { serving, sums, target } from "./fixtures";

const VERSION = "0.4.2";
const LINUX = target("linux-x86_64");
const ARCHIVE = archiveName(VERSION, LINUX);
const BYTES = Buffer.from("this stands in for a release archive");

async function refused(work: Promise<unknown>): Promise<string> {
  try {
    await work;
  } catch (thrown) {
    return (thrown as Error).message;
  }
  throw new Error("the download was expected to be refused and was not");
}

describe("the release a download comes from", () => {
  it("is addressed by the tag, which carries the v the archives do not", () => {
    assert.equal(
      assetUrl(VERSION, ARCHIVE),
      `https://github.com/novis-lang/novis/releases/download/v0.4.2/nvs-0.4.2-linux-x86_64.tar.gz`
    );
    assert.equal(assetUrl(VERSION, SUMS), "https://github.com/novis-lang/novis/releases/download/v0.4.2/SHA256SUMS");
  });
});

describe("the sums a release publishes", () => {
  const line = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

  it("are read out of sha256sum's own output", () => {
    assert.equal(expectedHash(`${line}  ${ARCHIVE}\n`, ARCHIVE), line);
  });

  it("are read the same when the file was hashed as binary", () => {
    assert.equal(expectedHash(`${line} *${ARCHIVE}\n`, ARCHIVE), line);
  });

  it("are upper case or lower case, and answer lower case", () => {
    assert.equal(expectedHash(`${line.toUpperCase()}  ${ARCHIVE}\n`, ARCHIVE), line);
  });

  it("name one file per line, so another archive's sum is not this one's", () => {
    const other = `${line}  nvs-0.4.2-macos-aarch64.tar.gz\n`;
    assert.equal(expectedHash(other, ARCHIVE), undefined);
    assert.equal(expectedHash(`${other}${line}  ${ARCHIVE}\n`, ARCHIVE), line);
  });

  it("are nothing at all when the line holds something that is not a digest", () => {
    assert.equal(expectedHash(`not-a-digest  ${ARCHIVE}\n`, ARCHIVE), undefined);
    assert.equal(expectedHash("", ARCHIVE), undefined);
  });
});

describe("a verified download", () => {
  it("hands back the archive when it matches the sum the release publishes", async () => {
    const release = serving({ [ARCHIVE]: BYTES, [SUMS]: sums({ [ARCHIVE]: BYTES }) });
    const got = await downloadVerified(VERSION, LINUX, release.transport);
    assert.deepEqual(Buffer.from(got), BYTES);
  });

  it("reads the sums first, so a release without this archive costs one small request", async () => {
    const release = serving({ [SUMS]: sums({ "nvs-0.4.2-macos-aarch64.tar.gz": BYTES }) });
    const why = await refused(downloadVerified(VERSION, LINUX, release.transport));
    assert.match(why, /publishes no sha256 for nvs-0\.4\.2-linux-x86_64\.tar\.gz/);
    assert.deepEqual(release.asked, [assetUrl(VERSION, SUMS)]);
  });

  it("keeps nothing when the archive does not match, and names the file that failed", async () => {
    const tampered = Buffer.concat([BYTES, Buffer.from("and one byte more")]);
    const release = serving({ [ARCHIVE]: tampered, [SUMS]: sums({ [ARCHIVE]: BYTES }) });
    const why = await refused(downloadVerified(VERSION, LINUX, release.transport));
    assert.match(why, /^nvs-0\.4\.2-linux-x86_64\.tar\.gz does not match the sha256/);
    assert.match(why, /Nothing was kept\./);
  });

  it("says which file the release does not have when the server answers with a status", async () => {
    const release = serving({ [SUMS]: sums({ [ARCHIVE]: BYTES }) });
    const why = await refused(downloadVerified(VERSION, LINUX, release.transport));
    assert.match(why, /nvs-0\.4\.2-linux-x86_64\.tar\.gz could not be downloaded from https:/);
    assert.match(why, /the server answered 404\./);
  });

  it("takes the newest release in the client's own series from the release list", async () => {
    const list = JSON.stringify([
      { tag_name: "v0.5.0" }, { tag_name: "v0.4.2" }, { tag_name: "v0.4.10" }, { name: "no tag" },
    ]);
    const release = serving({ releases: list });
    assert.equal(await releaseFor("0.4.0", release.transport), "0.4.10");
    assert.deepEqual(release.asked, [RELEASE_LIST]);
  });

  it("says so when the release list has nothing in the client's series", async () => {
    const release = serving({ releases: JSON.stringify([{ tag_name: "v0.5.0" }]) });
    const why = await refused(releaseFor("0.4.0", release.transport));
    assert.match(why, /^No release of nvs is in the 0\.4 series/);
  });

  it("says which file was being fetched when the network fails outright", async () => {
    const transport = () => Promise.reject(new Error("getaddrinfo ENOTFOUND github.com"));
    const why = await refused(downloadVerified(VERSION, LINUX, transport));
    assert.match(why, /^SHA256SUMS could not be downloaded from https:.*ENOTFOUND github\.com$/);
  });
});
