// SPDX-License-Identifier: Apache 2

pragma solidity ^0.8.0;

import "./interfaces/IWormhole.sol";
import "./BytesLib.sol";

/// @title WormholeLite is a lightweight contract provided clean access to the wormhole bridge
contract WormholeLite {
    using BytesLib for bytes;

    /// address of the deployer and contract owner
    address internal immutable deployer;
    /// address of the wormhole bridge contract deployed on the network
    address internal immutable wormholeBridgeContract;
    /// chain id of the network this contract is deployed on using the wormhole chainid standard
    uint16 internal immutable wormholeChainId;
    /// the number of confirmations needed for the wormhole network to attest to a message
    uint8 internal immutable wormholeFinality;

    /// an object that bundles a message to deliver via wormhole
    struct WormholeMessage {
        uint8 payloadId;
        bytes message;
    }

    /**
     * Wormhole chain ID to known emitter address mapping. xDapps using
     * Wormhole should register all deployed contracts on each chain to
     * verify that messages being consumed are from trusted contracts.
     */
    mapping(uint16 => bytes32) registeredEmitters;

    // verified message hash to received message mapping
    mapping(bytes32 => bytes) receivedMessages;

    // verified message hash to boolean
    mapping(bytes32 => bool) consumedMessages;

    constructor(address _wormholeBridgContract, uint16 _wormholeChainId, uint8 _wormholeFinality) {
        deployer = msg.sender;
        wormholeBridgeContract = _wormholeBridgContract;
        wormholeChainId = _wormholeChainId;
        wormholeFinality = _wormholeFinality;
    }
    /**
     * @notice Creates an arbitrary HelloWorld message to be attested by the
     * Wormhole guardians.
     * @dev batchID is set to 0 to opt out of batching in future Wormhole versions.
     * Reverts if:
     * - caller doesn't pass enough value to pay the Wormhole network fee
     * - `_message` length is >= max(uint16)
     * @param _message message to send
     * @return messageSequence Wormhole message sequence for this contract
     */

    function sendMessage(bytes calldata _message) public payable returns (uint64 messageSequence) {
        // enforce a max size for the arbitrary message
        require(abi.encodePacked(_message).length < type(uint16).max, "message too large");

        // cache Wormhole instance and fees to save on gas
        IWormhole wormhole = IWormhole(wormholeBridgeContract);
        uint256 wormholeFee = wormhole.messageFee();

        // Confirm that the caller has sent enough value to pay for the Wormhole
        // message fee.
        require(msg.value == wormholeFee, "insufficient value");

        // create the WormholeMessage struct
        WormholeMessage memory parsedMessage = WormholeMessage({payloadId: uint8(1), message: _message});

        // encode the WormholeMessage struct into bytes
        bytes memory encodedMessage = encodeMessage(parsedMessage);

        // Send the HelloWorld message by calling publishMessage on the
        // Wormhole core contract and paying the Wormhole protocol fee.
        messageSequence = wormhole.publishMessage{value: wormholeFee}(
            0, // batchID
            encodedMessage,
            wormholeFinality
        );
    }

    /**
     * @notice Consumes arbitrary HelloWorld messages sent by registered emitters
     * @dev The arbitrary message is verified by the Wormhole core endpoint
     * `verifyVM`.
     * Reverts if:
     * - `encodedMessage` is not attested by the Wormhole network
     * - `encodedMessage` was sent by an unregistered emitter
     * - `encodedMessage` was consumed already
     * @param encodedMessage verified Wormhole message containing arbitrary
     * HelloWorld message.
     */
    function receiveMessage(bytes memory encodedMessage) public {
        // call the Wormhole core contract to parse and verify the encodedMessage
        (IWormhole.VM memory wormholeMessage, bool valid, string memory reason) =
            IWormhole(wormholeBridgeContract).parseAndVerifyVM(encodedMessage);

        // confirm that the Wormhole core contract verified the message
        require(valid, reason);

        // verify that this message was emitted by a registered emitter
        require(verifyEmitter(wormholeMessage), "unknown emitter");

        // decode the message payload into the WormholeMessage struct
        WormholeMessage memory parsedMessage = decodeMessage(wormholeMessage.payload);

        /**
         * Check to see if this message has been consumed already. If not,
         * save the parsed message in the receivedMessages mapping.
         *
         * This check can protect against replay attacks in xDapps where messages are
         * only meant to be consumed once.
         */
        require(!isMessageConsumed(wormholeMessage.hash), "message already consumed");
        consumeMessage(wormholeMessage.hash, parsedMessage.message);
    }

    /**
     * @notice Registers foreign emitters (HelloWorld contracts) with this contract
     * @dev Only the deployer (owner) can invoke this method
     * @param emitterChainId Wormhole chainId of the contract being registered
     * See https://book.wormhole.com/reference/contracts.html for more information.
     * @param emitterAddress 32-byte address of the contract being registered. For EVM
     * contracts the first 12 bytes should be zeros.
     */
    function registerEmitter(uint16 emitterChainId, bytes32 emitterAddress) public {
        require(msg.sender == deployer);
        // sanity check the emitterChainId and emitterAddress input values
        require(
            emitterChainId != 0 && emitterChainId != wormholeChainId, "emitterChainId cannot equal 0 or this chainId"
        );
        require(emitterAddress != bytes32(0), "emitterAddress cannot equal bytes32(0)");

        // update the registeredEmitters state variable
        setEmitter(emitterChainId, emitterAddress);
    }

    function verifyEmitter(IWormhole.VM memory vm) internal view returns (bool) {
        // Verify that the sender of the Wormhole message is a trusted
        // HelloWorld contract.
        return getRegisteredEmitter(vm.emitterChainId) == vm.emitterAddress;
    }
    /**
     * @notice Encodes the WormholeMessage struct into bytes
     * @param parsedMessage WormholeMessage struct with arbitrary HelloWorld message
     * @return encodedMessage WormholeMessage encoded into bytes
     */

    function encodeMessage(WormholeMessage memory parsedMessage) public pure returns (bytes memory encodedMessage) {
        // Convert message string to bytes so that we can use the .length attribute.
        // The length of the arbitrary messages needs to be encoded in the message
        // so that the corresponding decode function can decode the message properly.
        bytes memory encodedMessagePayload = abi.encodePacked(parsedMessage.message);

        // return the encoded message
        encodedMessage =
            abi.encodePacked(parsedMessage.payloadId, uint16(encodedMessagePayload.length), encodedMessagePayload);
    }

    /**
     * @notice Decodes bytes into WormholeMessage struct
     * @dev Verifies the payloadID
     * @param encodedMessage encoded arbitrary HelloWorld message
     * @return parsedMessage WormholeMessage struct with arbitrary WormholeMessage message
     */
    function decodeMessage(bytes memory encodedMessage) public pure returns (WormholeMessage memory parsedMessage) {
        // starting index for byte parsing
        uint256 index = 0;

        // parse and verify the payloadID
        parsedMessage.payloadId = encodedMessage.toUint8(index);
        require(parsedMessage.payloadId == 1, "invalid payloadId");
        index += 1;

        // parse the message string length
        uint256 messageLength = encodedMessage.toUint16(index);
        index += 2;

        // parse the message string
        bytes memory messageBytes = encodedMessage.slice(index, messageLength);
        parsedMessage.message = messageBytes;
        index += messageLength;

        // confirm that the message was the expected length
        require(index == encodedMessage.length, "invalid message length");
    }

    function getRegisteredEmitter(uint16 emitterChainId) public view returns (bytes32) {
        return registeredEmitters[emitterChainId];
    }

    function isMessageConsumed(bytes32 hash) public view returns (bool) {
        return consumedMessages[hash];
    }

    function consumeMessage(bytes32 hash, bytes memory message) internal {
        receivedMessages[hash] = message;
        consumedMessages[hash] = true;
    }

    function setEmitter(uint16 chainId, bytes32 emitter) internal {
        registeredEmitters[chainId] = emitter;
    }
}
