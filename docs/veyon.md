# Veyon lab adapter

SkillBridge consumes student consent through its HTTPS API. The teacher's local adapter runs the official [Veyon CLI](https://docs.veyon.io/en/latest/admin/cli.html#remote-access) on the teacher workstation. Screen data and keyboard input travel between Veyon and the lab computer on the LAN. SkillBridge records who consumed consent and when, without receiving screen contents, Veyon keys, or lab IPs in the backend. This integration is off by default.

## Configure a lab

1. Install Veyon independently on the teacher and lab computers. Configure Veyon authentication and limit authorized teachers and device networks. Complete the lab checklist below before opting in.
2. In SkillBridge, create a class and enroll the student. Assign the student's physical lab seat as a database operator. Use the displayed class ID and record the returned seat ID.

   ```sh
   docker compose run --rm api assign-seat CLASS_UUID student@example.test "PC Lab 01"
   ```

3. Build the SkillBridge binary on the teacher workstation, for example with `cargo build --locked --release --manifest-path backend/Cargo.toml`. Save a trusted local file named `veyon.local.toml` on that workstation. Map assigned seat UUIDs to confirmed RFC 1918 IPv4 addresses. The adapter accepts only private IP literals and uses the standard Veyon port. Replace the example address, ID and executable path.

   ```toml
   origin = "https://skillbridge.example.test"
   veyon_cli = "/usr/bin/veyon-cli"

   [seats]
   "a20f5972-d77e-434a-88e5-d3e7e119614c" = "192.168.50.12"
   ```

   On Windows, a TOML single-quoted value can contain a path like `'C:\Program Files\Veyon\veyon-cli.exe'`. The API origin must use HTTPS with a certificate the client trusts, except for a local demo at `http://127.0.0.1:8080`. Keep the config and executable writable only by trusted workstation administrators.

4. Set `REMOTE_ASSISTANCE=true` and recreate the API service with `docker compose up -d --wait`. The student confirms an opt-in for the assigned seat when opening a help request. The offer expires after ten minutes. Closing the ticket cancels unused consent.
5. Open the ticket as its teacher and expand the Veyon section. Run the command below from the teacher's interactive desktop. On Windows run `skillbridge.exe`. Check the device and the student before typing the launch confirmation. The adapter prompts for the instructor password without putting it in the command line.

   ```sh
   skillbridge veyon --config veyon.local.toml --email teacher@example.test --ticket TICKET_UUID --seat SEAT_UUID
   ```

The server consumes the offer once, atomically, before the helper launches Veyon. The timestamp records that claim. Successful connection is confirmed only in the Veyon window. If launch or the network fails after the claim, start a fresh request with student consent. To stop a live session, close the Veyon window. Closing the SkillBridge ticket affects future claims and has no control over a Veyon window already open.

## Lab checklist

- Restrict the [Veyon server port](https://docs.veyon.io/en/latest/admin/reference.html#network-port-numbers), normally TCP 11100, to designated teacher workstations on the lab LAN. Remove broad installer firewall exceptions. Do not port-forward it, publish it with Docker, or route Veyon traffic through SkillBridge.
- Configure [Veyon access rules](https://docs.veyon.io/en/latest/admin/access-control-rules.html) for authorized teachers and use the action that asks the logged-on user for permission. Test acceptance, rejection and unauthorized workstation access before using real student accounts.
- Keep the local seat map accurate and unique. Test that the assigned seat matches the person using the computer, and check remote disconnection from the actual lab workstation.

The automated tests exercise the API consent and command boundary. The graphical session and firewall checklist require an actual Veyon lab.

## License boundary

SkillBridge's first-party code is MIT. The helper passes a fixed command and private address as process arguments to an independently installed Veyon executable. SkillBridge does not bundle, link, copy or redistribute Veyon code, binaries or plugins. Veyon's upstream [COPYING](https://github.com/veyon/veyon/blob/main/COPYING) contains GPL version 2 and its upstream OpenSSL exception. Commercial add-ons are outside this integration. Review applicable upstream terms and distribution obligations before bundling, embedding, modifying or redistributing any Veyon component.
