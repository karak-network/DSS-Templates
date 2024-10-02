// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import "forge-std/console.sol";
import {IDSS} from "./karak/src/interfaces/IDSS.sol";
import {ICore} from "./karak/src/interfaces/ICore.sol";
import {Operator} from "./karak/src/entities/Operator.sol";
import {BlsSdk, BN254} from "./libraries/BlsSdk.sol";

contract SquareNumberDSS {
    using BN254 for BN254.G1Point;

    struct TaskRequest {
        uint256 value;
    }

    struct TaskResponse {
        uint256 response;
    }

    constructor(address _aggregator, ICore _core) {
        aggregator = _aggregator;
        core = _core;
    }

    /* ======= State Variables ======= */

    address aggregator;
    mapping(bytes32 taskRequestHash => bool exists) taskExists;
    mapping(bytes32 taskRequestHash => bool completed) taskCompleted;
    mapping(bytes32 taskRequestHash => TaskResponse taskResponse) taskResponses;
    ICore core;

    BlsSdk.State blsState;

    // keccak of "Register to square number dss"
    bytes32 public constant REGISTRATION_MESSAGE_HASH =
        bytes32(
            0xafd770cae74215647d508372fe8c5b866178892133d2611c6ec8b4f479fa0680
        );

    /* ======= Events ======= */

    event TaskRequestGenerated(address sender, TaskRequest taskRequest);
    event TaskResponseSubmitted(TaskResponse taskResponse);

    /* ======= External Functions ======= */

    function registerDSS(uint256 wadPercentage) external {
        core.registerDSS(wadPercentage);
    }

    function slashOperator(address operator, uint256 index) external {}

    function generateTaskRequest(TaskRequest calldata taskRequest) external {
        bytes32 taskRequestHash = keccak256(abi.encode(taskRequest));
        if (taskExists[taskRequestHash]) revert TaskAlreadyExists();
        taskExists[taskRequestHash] = true;
        emit TaskRequestGenerated(msg.sender, taskRequest);
    }

    /* ======= Hooks ======= */

    function supportsInterface(
        bytes4 interfaceID
    ) external view returns (bool) {
        if (
            interfaceID == IDSS.registrationHook.selector ||
            interfaceID == IDSS.unregistrationHook.selector
        ) {
            return true;
        }
        return false;
    }

    function registrationHook(
        address operator,
        bytes memory extraData
    ) external senderIsOperator(operator) {
        BlsSdk.operatorRegistration(
            blsState,
            operator,
            extraData,
            REGISTRATION_MESSAGE_HASH
        );
    }

    function unregistrationHook(
        address operator,
        bytes memory extraData
    ) external senderIsOperator(operator) {
        BlsSdk.operatorUnregistration(blsState, operator);
    }

    function requestUpdateStakeHook(
        address operator,
        Operator.StakeUpdateRequest memory newStake
    ) external {}
    function cancelUpdateStakeHook(address operator, address vault) external {}
    function finishUpdateStakeHook(address operator) external {}
    function requestSlashingHook(
        address operator,
        uint256[] memory slashingPercentagesWad
    ) external {}
    function cancelSlashingHook(address operator) external {}
    function finishSlashingHook(address operator) external {}

    /* ======= Only Aggregator Functions ======= */

    function submitTaskResponse(
        TaskRequest calldata taskRequest,
        TaskResponse calldata taskResponse,
        BN254.G1Point[] calldata nonSigningOperators,
        BN254.G2Point calldata aggG2Pubkey,
        BN254.G1Point calldata aggSign
    ) external onlyAggregator {
        if (
            nonSigningOperators.length >
            (blsState.allOperatorPubkeyG1.length / 2)
        ) revert NotEnoughOperatorsForMajority();
        bytes32 taskReqeustHash = keccak256(abi.encode(taskRequest));
        taskCompleted[taskReqeustHash] = true;
        taskResponses[taskReqeustHash] = taskResponse;

        BN254.G1Point memory nonSigningAggG1Key = BN254.G1Point(0, 0);
        for (uint256 i = 0; i < nonSigningOperators.length; i++) {
            nonSigningAggG1Key = nonSigningAggG1Key.plus(
                nonSigningOperators[i]
            );
        }
        nonSigningAggG1Key = nonSigningAggG1Key.negate();
        //calculated G1 pubkey
        BN254.G1Point memory calculatedG1Pubkey = blsState
            .aggregatedG1Pubkey
            .plus(nonSigningAggG1Key);

        BlsSdk.verifySignature(
            calculatedG1Pubkey,
            aggG2Pubkey,
            aggSign,
            msgToHash(taskResponse)
        );

        emit TaskResponseSubmitted(taskResponse);
    }

    /* ======= View Functions ======= */

    function getTaskResponse(
        TaskRequest calldata taskRequest
    ) external view returns (TaskResponse memory) {
        bytes32 taskRequestHash = keccak256(abi.encode(taskRequest));
        return taskResponses[taskRequestHash];
    }

    function isTaskCompleted(
        TaskRequest calldata taskRequest
    ) external view returns (bool) {
        bytes32 taskRequestHash = keccak256(abi.encode(taskRequest));
        return taskCompleted[taskRequestHash];
    }

    function isOperatorRegistered(
        address operator
    ) external view returns (bool) {
        return BlsSdk.isOperatorRegistered(blsState, operator);
    }

    function msgToHash(
        TaskResponse calldata taskResponse
    ) public pure returns (bytes32) {
        return keccak256(abi.encode(taskResponse));
    }

    function allOperatorsG1() external view returns (BN254.G1Point[] memory) {
        return BlsSdk.allOperatorsG1(blsState);
    }

    /* ======= Modifiers ======= */

    modifier onlyAggregator() {
        if (msg.sender != aggregator) revert NotAggregator();
        _;
    }

    modifier onlyCore() {
        if (msg.sender != address(core)) revert NotCore();
        _;
    }

    modifier senderIsOperator(address operator) {
        if (tx.origin != operator) revert SenderNotOperator();
        _;
    }

    /* ======= Errors ======= */
    error OperatorAlreadyRegistered();
    error OperatorAndIndexDontMatch();
    error OperatorIsNotRegistered();
    error NotAggregator();
    error TaskAlreadyExists();
    error SenderNotOperator();
    error SignatureVerificationFailed();
    error PairingNotSuccessful();
    error NotEnoughOperatorsForMajority();
    error NotCore();
}
