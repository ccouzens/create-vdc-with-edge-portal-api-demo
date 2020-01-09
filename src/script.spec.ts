import { setupMocha as setupPolly } from "@pollyjs/core";
import * as NodeHttpAdapter from "@pollyjs/adapter-node-http";
import * as FSPersister from "@pollyjs/persister-fs";
import script from "./script";

describe("Array", function() {
  setupPolly({
    recordFailedRequests: true,
    adapters: [NodeHttpAdapter as any],
    persister: FSPersister as any
  });
  describe("#indexOf()", function() {
    it("should return -1 when the value is not present", async function() {
      await script(
        "https://portal.skyscapecloud.com",
        "https://api.vcd.pod0000b.sys00005.portal.skyscapecloud.com",
        "???",
        "???",
        "???",
        "???",
        "???",
        _message => {}
      );
    });
  });
});
