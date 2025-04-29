// SPDX-License-Identifier: MIT
pragma solidity 0.8.22;

import {Script} from "forge-std/Script.sol";
import {console2 as console} from "forge-std/console2.sol";

/**
 * @title StacTrigger
 * @dev Script to trigger a STAC query by calling the oracle service contract
 */
contract StacTrigger is Script {
    /**
     * @notice Runs the script with the provided parameters
     * @param _triggerContract Address of the trigger contract
     * @param _stacQuery The STAC query as a JSON string
     */
    function run(string memory _triggerContract, string memory _stacQuery) public {
        // Parse trigger contract address
        address triggerContract = vm.parseAddress(_triggerContract);
        console.log("Using trigger contract at:", triggerContract);
        
        // Convert the STAC query string to bytes
        bytes memory stacQueryBytes = bytes(_stacQuery);
        console.log("STAC Query length:", stacQueryBytes.length);
        
        // Send the query to the contract (this will emit an event picked up by WAVS)
        vm.broadcast();
        (bool success, ) = triggerContract.call(
            abi.encodeWithSignature("addTrigger(bytes)", stacQueryBytes)
        );

        if (!success) {
            console.log("Failed to trigger STAC query");
            revert("Transaction failed");
        }

        console.log("Successfully triggered STAC query");
    }
}