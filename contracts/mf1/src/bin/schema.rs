use mf1::msg::*;
use cosmwasm_schema::write_api;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        execute: ExecuteMsg,
        query: QueryMsg
    }

    // generate a valid json
    let msg = ExecuteMsg::ExecuteTx {
        fcross_tx: FcrossTx {
            tx_id: 1,
            operation: Operation::CreditBalance { amount: 100 },
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    println!("{}", json);
    let msg2 = ExecuteMsg::FinalizeTx {
        tx_info: TxInfo{
            tx_id: 2,
            committed: false,
        }
    };
    let json2 = serde_json::to_string(&msg2).unwrap();
    println!("{}", json2);
}
