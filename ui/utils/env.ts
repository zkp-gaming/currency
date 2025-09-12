import { Principal } from "@dfinity/principal";
import { IsDev } from "@zk-game-dao/ui";

export const IC_SIWB_ID = Principal.fromText(
  IsDev ? "be2us-64aaa-aaaaa-qaabq-cai" : "j2let-saaaa-aaaam-qds2q-cai"
);

export const IC_SIWS_ID = Principal.fromText(
  IsDev ? "uxrrr-q7777-77774-qaaaq-cai" : "uxrrr-q7777-77774-qaaaq-cai"
);
